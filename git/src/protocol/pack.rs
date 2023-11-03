//!
//!
//!
//!

use crate::protocol::{RefsType, ZERO_ID};
use crate::structure::conversion;
use crate::{
    errors::GitError,
    internal::pack::{
        decode::HashCounter,
        preload::{decode_load, PackPreload},
    },
};
use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use database::driver::ObjectStorage;
use std::io::Write;
use std::{collections::HashSet, env, io::Cursor, path::PathBuf, sync::Arc, thread};

use super::{new_mr_info, Capability, PackProtocol, Protocol, RefCommand, ServiceType, SideBind};

const LF: char = '\n';

pub const SP: char = ' ';

const NUL: char = '\0';

pub const PKT_LINE_END_MARKER: &[u8; 4] = b"0000";

// The atomic, report-status, report-status-v2, delete-refs, quiet,
// and push-cert capabilities are sent and recognized by the receive-pack (push to server) process.
const RECEIVE_CAP_LIST: &str = "report-status report-status-v2 delete-refs quiet atomic ";

// The ofs-delta and side-band-64k capabilities are sent and recognized by both upload-pack and receive-pack protocols.
// The agent and session-id capabilities may optionally be sent in both protocols.
const CAP_LIST: &str = "side-band-64k ofs-delta";

// All other capabilities are only recognized by the upload-pack (fetch from server) process.
const UPLOAD_CAP_LIST: &str =
    "shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done include-tag ";

impl PackProtocol {
    /// # Retrieves the information about Git references (refs) for the specified service type.
    ///
    /// The function returns a `BytesMut` object containing the Git reference information.
    ///
    /// The `service_type` is extracted from the `PackProtocol` instance.
    ///
    /// The function checks if the `object_id` of the head object in the storage is zero. If it is zero,
    /// the name is set to "capabilities^{}" to include capability declarations behind a NUL on the first ref.
    /// Otherwise, the name is set to "HEAD".
    ///
    /// The `cap_list` is determined based on the `service_type` and contains the appropriate capability lists.
    ///
    /// A packet line (`pkt_line`) is constructed using the `object_id`, `name`, `NUL` delimiter, `cap_list`, and line feed (`LF`).
    /// The `pkt_line` is added to the `ref_list`.
    ///
    /// The `object_id` and `name` pairs for other refs are retrieved using the `get_ref_object_id` method from the storage.
    /// Each pair is used to construct a packet line (`pkt_line`) and added to the `ref_list`.
    ///
    /// The `build_smart_reply` method is called with the `ref_list`, `service_type`, and its string representation
    /// to build a smart reply packet line stream.
    ///
    /// Tracing information is logged regarding the response packet line stream.
    ///
    /// Finally, the constructed packet line stream is returned.
    pub async fn git_info_refs(&mut self, service_type: ServiceType) -> BytesMut {
        // The stream MUST include capability declarations behind a NUL on the first ref.
        let object_id = self.get_head_object_id(&self.path).await;
        let name = if object_id == ZERO_ID {
            "capabilities^{}"
        } else {
            "HEAD"
        };
        let cap_list = match service_type {
            ServiceType::UploadPack => format!("{}{}", UPLOAD_CAP_LIST, CAP_LIST),
            ServiceType::ReceivePack => format!("{}{}", RECEIVE_CAP_LIST, CAP_LIST),
            // _ => CAP_LIST.to_owned(),
        };
        let pkt_line = format!("{}{}{}{}{}{}", object_id, SP, name, NUL, cap_list, LF);
        let mut ref_list = vec![pkt_line];

        let git_refs = self
            .storage
            .get_all_refs_by_path(self.path.to_str().unwrap())
            .await
            .unwrap();
        for git_ref in git_refs {
            let pkt_line = format!("{}{}{}{}", git_ref.ref_git_id, SP, git_ref.ref_name, LF);
            ref_list.push(pkt_line);
        }
        let pkt_line_stream = self.build_smart_reply(&ref_list, service_type.to_string());
        tracing::debug!("git_info_refs response: {:?}", pkt_line_stream);
        pkt_line_stream
    }

    pub async fn git_upload_pack(
        &mut self,
        upload_request: &mut Bytes,
    ) -> Result<(Vec<u8>, BytesMut)> {
        let mut want: HashSet<String> = HashSet::new();
        let mut have: HashSet<String> = HashSet::new();

        let mut read_first_line = false;
        loop {
            tracing::debug!("loop start");
            let (bytes_take, pkt_line) = read_pkt_line(upload_request);
            // read 0000 to continue and read empty str to break
            if bytes_take == 0 {
                if upload_request.is_empty() {
                    break;
                } else {
                    continue;
                }
            }
            tracing::debug!("read line: {:?}", pkt_line);
            let dst = pkt_line.to_vec();
            let commands = &dst[0..4];

            match commands {
                b"want" => want.insert(String::from_utf8(dst[5..45].to_vec()).unwrap()),
                b"have" => have.insert(String::from_utf8(dst[5..45].to_vec()).unwrap()),
                b"done" => break,
                other => {
                    tracing::error!(
                        "unsupported command: {:?}",
                        String::from_utf8(other.to_vec())
                    );
                    continue;
                }
            };
            if !read_first_line {
                self.parse_capabilities(&String::from_utf8(dst[46..].to_vec()).unwrap());
                read_first_line = true;
            }
        }

        tracing::info!(
            "want commands: {:?}\n have commans: {:?}\n caps:{:?}",
            want,
            have,
            self.capabilities
        );

        let mut send_pack_data = vec![];
        let mut buf = BytesMut::new();

        if have.is_empty() {
            send_pack_data = self.get_full_pack_data(&self.path).await.unwrap();
            add_pkt_line_string(&mut buf, String::from("NAK\n"));
        } else {
            if self.capabilities.contains(&Capability::MultiAckDetailed) {
                // multi_ack_detailed mode, the server will differentiate the ACKs where it is signaling that
                // it is ready to send data with ACK obj-id ready lines,
                // and signals the identified common commits with ACK obj-id common lines
                for hash in &have {
                    if self.storage.get_commit_by_hash(hash).await.is_ok() {
                        add_pkt_line_string(&mut buf, format!("ACK {} common\n", hash));
                    }
                    // no need to send NAK in this mode if missing commit?
                }

                send_pack_data = self
                    .get_incremental_pack_data(&self.path, &want, &have)
                    .await
                    .unwrap();

                for hash in &want {
                    if self.storage.get_commit_by_hash(hash).await.is_ok() {
                        add_pkt_line_string(&mut buf, format!("ACK {} common\n", hash));
                    }
                    if self.capabilities.contains(&Capability::NoDone) {
                        // If multi_ack_detailed and no-done are both present, then the sender is free to immediately send a pack
                        // following its first "ACK obj-id ready" message.
                        add_pkt_line_string(&mut buf, format!("ACK {} ready\n", hash));
                    }
                }
            } else {
                tracing::error!("capability unsupported");
            }
            // TODO: hard-code here
            add_pkt_line_string(
                &mut buf,
                format!("ACK {} \n", "27dd8d4cf39f3868c6eee38b601bc9e9939304f5"),
            );
        }
        Ok((send_pack_data, buf))
    }

    pub async fn git_receive_pack(&mut self, mut body_bytes: Bytes) -> Result<Bytes> {
        if body_bytes.len() < 1000 {
            tracing::debug!("bytes from client: {:?}", body_bytes);
        }
        while !body_bytes.starts_with(&[b'P', b'A', b'C', b'K']) && !body_bytes.is_empty() {
            let (bytes_take, mut pkt_line) = read_pkt_line(&mut body_bytes);
            if bytes_take != 0 {
                let command = self.parse_ref_command(&mut pkt_line);
                self.parse_capabilities(&String::from_utf8(pkt_line.to_vec()).unwrap());
                tracing::debug!("init command: {:?}, caps:{:?}", command, self.capabilities);
                self.command_list.push(command);
            }
        }
        // handles situation when client send b"0000"
        if body_bytes.is_empty() {
            return Ok(body_bytes);
        }
        // After receiving the pack data from the sender, the receiver sends a report
        let mut report_status = BytesMut::new();

        //1. unpack progress
        let mr_id = unpack(self.storage.clone(), &mut body_bytes).await?;
        // write "unpack ok\n to report"
        add_pkt_line_string(&mut report_status, "unpack ok\n".to_owned());
        //2. parse progress
        let parse_obj_result =
            conversion::save_node_from_mr(self.storage.clone(), mr_id, &self.path)
                .await
                .is_ok();

        //3. update each refs and build report
        for mut command in self.command_list.clone() {
            if command.refs_type == RefsType::Tag {
                // just update if refs type is tag
                command.update_refs(self.storage.clone(), &self.path).await;
            } else {
                // TODO: Updates can be unsuccessful for a number of reasons.
                // a.The reference can have changed since the reference discovery phase was originally sent, meaning someone pushed in the meantime.
                // b.The reference being pushed could be a non-fast-forward reference and the update hooks or configuration could be set to not allow that, etc.
                // c.Also, some references can be updated while others can be rejected.
                if parse_obj_result {
                    command.update_refs(self.storage.clone(), &self.path).await;
                    self.handle_directory().await.unwrap();
                    trigger_build(self.path.clone())
                } else {
                    command.failed(String::from("parse commit tree from obj failed"));
                }
            }
            add_pkt_line_string(&mut report_status, command.get_status());
        }
        report_status.put(&PKT_LINE_END_MARKER[..]);
        let length = report_status.len();
        let mut buf = self.build_side_band_format(report_status, length);
        buf.put(&PKT_LINE_END_MARKER[..]);
        Ok(buf.into())
    }

    /// # Builds the packet data in the sideband format if the SideBand/64k capability is enabled.
    ///
    /// If the `SideBand` or `SideBand64k` capability is present in the `capabilities` vector,
    /// the `from_bytes` data is transformed into the sideband format.
    /// The resulting packet data is returned in a `BytesMut` object.
    ///
    /// The `length` parameter represents the length of the `from_bytes` data.
    /// It is used to calculate the length of the transformed packet data.
    ///
    /// If the sideband format is enabled, the resulting packet data is constructed as follows:
    /// - The length of the packet data (including header) is calculated by adding 5 to the `length`.
    /// - The length value is formatted as a hexadecimal string and prepended to the `to_bytes` buffer.
    /// - The sideband type (`PackfileData`) is added as a single byte to the `to_bytes` buffer.
    /// - The `from_bytes` data is appended to the `to_bytes` buffer.
    /// - The `to_bytes` buffer containing the transformed packet data is returned.
    ///
    /// If the sideband format is not enabled, the `from_bytes` data is returned unchanged.
    pub fn build_side_band_format(&self, from_bytes: BytesMut, length: usize) -> BytesMut {
        let capabilities = &self.capabilities;
        if capabilities.contains(&Capability::SideBand)
            || capabilities.contains(&Capability::SideBand64k)
        {
            let mut to_bytes = BytesMut::new();
            let length = length + 5;
            to_bytes.put(Bytes::from(format!("{length:04x}")));
            to_bytes.put_u8(SideBind::PackfileData.value());
            to_bytes.put(from_bytes);
            return to_bytes;
        }
        from_bytes
    }

    pub fn build_smart_reply(&self, ref_list: &Vec<String>, service: String) -> BytesMut {
        let mut pkt_line_stream = BytesMut::new();
        if self.protocol == Protocol::Http {
            add_pkt_line_string(&mut pkt_line_stream, format!("# service={}\n", service));
            pkt_line_stream.put(&PKT_LINE_END_MARKER[..]);
        }

        for ref_line in ref_list {
            add_pkt_line_string(&mut pkt_line_stream, ref_line.to_string());
        }
        pkt_line_stream.put(&PKT_LINE_END_MARKER[..]);
        pkt_line_stream
    }

    pub fn parse_capabilities(&mut self, cap_str: &str) {
        let cap_vec: Vec<_> = cap_str.split(' ').collect();
        for cap in cap_vec {
            let res = cap.trim().parse::<Capability>();
            if let Ok(cap) = res {
                self.capabilities.push(cap);
            }
        }
    }

    // the first line contains the capabilities
    pub fn parse_ref_command(&self, pkt_line: &mut Bytes) -> RefCommand {
        RefCommand::new(
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
        )
    }
}

pub async fn unpack(
    storage: Arc<dyn ObjectStorage>,
    pack_file: &mut Bytes,
) -> Result<i64, GitError> {
    let count_hash: bool = true;
    //ONLY FOR TEST .NEED TO DELETE
    {
        let path = "lines.pack";
        let mut output = std::fs::File::create(path).unwrap();
        output.write_all(pack_file).unwrap();
    }
   

    let curosr_pack = Cursor::new(pack_file);
    let reader = HashCounter::new(curosr_pack, count_hash);
    let p = PackPreload::new(reader);
    let mr_id = decode_load(p, storage.clone()).await?;
    storage.save_mr_info(new_mr_info(mr_id)).await.unwrap();
    Ok(mr_id)
}
pub fn trigger_build(repo_path: PathBuf) {
    let enable_build = env::var("BAZEL_BUILD_ENABLE")
        .unwrap()
        .parse::<bool>()
        .unwrap();
    if enable_build {
        thread::spawn(|| build_bazel_tool::bazel_build::build(repo_path));
    }
}

fn read_until_white_space(bytes: &mut Bytes) -> String {
    let mut buf = Vec::new();
    while bytes.has_remaining() {
        let c = bytes.get_u8();
        if c.is_ascii_whitespace() || c == 0 {
            break;
        }
        buf.push(c);
    }
    String::from_utf8(buf).unwrap()
}

fn add_pkt_line_string(pkt_line_stream: &mut BytesMut, buf_str: String) {
    let buf_str_length = buf_str.len() + 4;
    pkt_line_stream.put(Bytes::from(format!("{buf_str_length:04x}")));
    pkt_line_stream.put(buf_str.as_bytes());
}
/// Read a single pkt-format line from the `bytes` buffer and return the line length and line bytes.
///
/// If the `bytes` buffer is empty, indicating no more data is available, the function returns a line length of 0 and an empty `Bytes` object.
///
/// The pkt-format line consists of a 4-byte length field followed by the line content. The length field specifies the total length of the line, including the length field itself. The line content is returned as a `Bytes` object.
///
/// The function first reads the 4-byte length field from the `bytes` buffer. The length value is then parsed as a hexadecimal string and converted into a `usize` value.
///
/// If the resulting line length is 0, indicating an empty line, the function returns a line length of 0 and an empty `Bytes` object.
///
/// If the line length is non-zero, the function extracts the line content from the `bytes` buffer. The extracted line content is returned as a `Bytes` object.
/// Note that this operation modifies the `bytes` buffer, consuming the bytes up to the end of the line.
///
/// # Arguments
///
/// * `bytes` - A mutable reference to a `Bytes` object representing the buffer containing pkt-format data.
///
/// # Returns
///
/// A tuple `(usize, Bytes)` representing the line length and line bytes respectively. If there is no more data available in the `bytes` buffer, the line length is 0 and an empty `Bytes` object is returned.
///
/// # Examples
///
/// ```
/// use bytes::Bytes;
/// use git::protocol::pack::read_pkt_line;
///
/// let mut bytes = Bytes::from_static(b"000Bexample");
/// let (length, line) = read_pkt_line(&mut bytes);
/// assert_eq!(length, 11);
/// assert_eq!(line, Bytes::from_static(b"example"));
/// ```
pub fn read_pkt_line(bytes: &mut Bytes) -> (usize, Bytes) {
    if bytes.is_empty() {
        return (0, Bytes::new());
    }
    let pkt_length = bytes.copy_to_bytes(4);
    let pkt_length =
        usize::from_str_radix(&String::from_utf8(pkt_length.to_vec()).unwrap(), 16).unwrap();

    if pkt_length == 0 {
        return (0, Bytes::new());
    }
    // this operation will change the original bytes
    let pkt_line = bytes.copy_to_bytes(pkt_length - 4);

    (pkt_length, pkt_line)
}

#[cfg(test)]
pub mod test {
    use bytes::{Bytes, BytesMut};

    use crate::protocol::{Capability, CommandType, PackProtocol, RefCommand, RefsType};

    use super::{add_pkt_line_string, read_pkt_line, read_until_white_space};

    #[test]
    pub fn test_read_pkt_line() {
        let mut bytes = Bytes::from_static(b"001e# service=git-upload-pack\n");
        let (pkt_length, pkt_line) = read_pkt_line(&mut bytes);
        assert_eq!(pkt_length, 30);
        assert_eq!(&pkt_line[..], b"# service=git-upload-pack\n");
    }

    #[test]
    pub fn test_build_smart_reply() {
        let mock = PackProtocol::mock();
        let ref_list = vec![String::from("7bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n")];
        let pkt_line_stream = mock.build_smart_reply(&ref_list, String::from("git-upload-pack"));
        assert_eq!(&pkt_line_stream[..], b"001e# service=git-upload-pack\n000000e87bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n0000")
    }

    #[test]
    pub fn test_add_to_pkt_line() {
        let mut buf = BytesMut::new();
        add_pkt_line_string(
            &mut buf,
            format!(
                "ACK {} common\n",
                "7bdc783132575d5b3e78400ace9971970ff43a18"
            ),
        );
        add_pkt_line_string(
            &mut buf,
            format!("ACK {} ready\n", "7bdc783132575d5b3e78400ace9971970ff43a18"),
        );
        assert_eq!(&buf.freeze()[..], b"0038ACK 7bdc783132575d5b3e78400ace9971970ff43a18 common\n0037ACK 7bdc783132575d5b3e78400ace9971970ff43a18 ready\n");
    }

    #[test]
    pub fn test_read_until_white_space() {
        let mut bytes = Bytes::from("Mega - A Monorepo Platform Engine".as_bytes());
        let result = read_until_white_space(&mut bytes);
        assert_eq!(result, "Mega");

        let mut bytes = Bytes::from("Hello,World!".as_bytes());
        let result = read_until_white_space(&mut bytes);
        assert_eq!(result, "Hello,World!");

        let mut bytes = Bytes::from("".as_bytes());
        let result = read_until_white_space(&mut bytes);
        assert_eq!(result, "");
    }

    #[test]
    pub fn test_parse_ref_update() {
        let mock = PackProtocol::mock();
        let mut bytes = Bytes::from("0000000000000000000000000000000000000000 27dd8d4cf39f3868c6eee38b601bc9e9939304f5 refs/heads/master\0".as_bytes());
        let result = mock.parse_ref_command(&mut bytes);

        let command = RefCommand {
            ref_name: String::from("refs/heads/master"),
            old_id: String::from("0000000000000000000000000000000000000000"),
            new_id: String::from("27dd8d4cf39f3868c6eee38b601bc9e9939304f5"),
            status: String::from("ok"),
            error_msg: String::new(),
            command_type: CommandType::Create,
            refs_type: RefsType::default(),
        };
        assert_eq!(result, command);
    }

    #[test]
    pub fn test_parse_capabilities() {
        let mut mock = PackProtocol::mock();
        mock.parse_capabilities("report-status-v2 side-band-64k object-format=sha10000");
        assert_eq!(
            mock.capabilities,
            vec![Capability::ReportStatusv2, Capability::SideBand64k]
        );
    }
}
