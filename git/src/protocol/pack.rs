//!
//!
//!
//!
use std::path::Path;
use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use database::driver::ObjectStorage;
use entity::commit;

use crate::internal::pack::decode::ObjDecodedMap;
use crate::protocol::ZERO_ID;
use crate::structure::nodes::build_node_tree;

use super::{Capability, CommandType, PackProtocol, Protocol, RefCommand, ServiceType, SideBind};

const LF: char = '\n';

pub const SP: char = ' ';

const NUL: char = '\0';

pub const PKT_LINE_END_MARKER: &[u8; 4] = b"0000";

// The atomic, report-status, report-status-v2, delete-refs, quiet,
// and push-cert capabilities are sent and recognized by the receive-pack (push to server) process.
const RECEIVE_CAP_LIST: &str = "report-status report-status-v2 delete-refs quiet atomic ";

// The ofs-delta and side-band-64k capabilities are sent and recognized by both upload-pack and receive-pack protocols.
// The agent and session-id capabilities may optionally be sent in both protocols.
const CAP_LIST: &str = "side-band-64k ofs-delta object-format=sha1";

// All other capabilities are only recognized by the upload-pack (fetch from server) process.
const UPLOAD_CAP_LIST: &str =
    "shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done ";

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
        let object_id = self.storage.get_head_object_id(&self.path).await;
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

        let obj_ids = self.storage.get_ref_object_id(&self.path).await;
        for (object_id, name) in obj_ids {
            let pkt_line = format!("{}{}{}{}", object_id, SP, name, LF);
            ref_list.push(pkt_line);
        }
        let pkt_line_stream = self.build_smart_reply(&ref_list, service_type.to_string());
        tracing::info!("git_info_refs response: {:?}", pkt_line_stream);
        pkt_line_stream
    }

    pub async fn git_upload_pack(
        &mut self,
        upload_request: &mut Bytes,
    ) -> Result<(Vec<u8>, BytesMut)> {
        let mut want: HashSet<String> = HashSet::new();
        let mut have: HashSet<String> = HashSet::new();

        let mut first_line = true;
        loop {
            let (bytes_take, pkt_line) = read_pkt_line(upload_request);
            // if read 0000
            if bytes_take == 0 && pkt_line.is_empty() {
                continue;
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
            if first_line {
                self.parse_capabilities(&String::from_utf8(dst[46..].to_vec()).unwrap());
                first_line = false;
            }
        }

        tracing::info!(
            "want commands: {:?}, have commans: {:?}, caps:{:?}",
            want,
            have,
            self.capabilities
        );

        let mut send_pack_data = vec![];
        let mut buf = BytesMut::new();

        if have.is_empty() {
            send_pack_data = self.storage.get_full_pack_data(&self.path).await.unwrap();
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
                    .storage
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

        if body_bytes.starts_with(&[b'P', b'A', b'C', b'K']) {
            let command = self.command_list.last_mut().unwrap();
            let object_map = command.unpack(&mut body_bytes).await.unwrap();
            let path = &self.path;
            // let storgae = self.storage.clone();
            let pack_result = save_packfile(self.storage.clone(), object_map, path).await;
            if pack_result.is_ok() {
                handle_refs(self.storage.clone(), command, path).await;
            } else {
                tracing::error!("{}", pack_result.err().unwrap());
                command.failed(String::from("db operation failed"));
            }
            // After receiving the pack data from the sender, the receiver sends a report
            let mut report_status = BytesMut::new();
            // TODO: replace this hard code "unpack ok\n"
            add_pkt_line_string(&mut report_status, "unpack ok\n".to_owned());
            for command in &self.command_list {
                add_pkt_line_string(&mut report_status, command.get_status());
            }
            report_status.put(&PKT_LINE_END_MARKER[..]);

            let length = report_status.len();
            let mut buf = self.build_side_band_format(report_status, length);
            buf.put(&PKT_LINE_END_MARKER[..]);
            Ok(buf.into())
        } else {
            let (bytes_take, mut pkt_line) = read_pkt_line(&mut body_bytes);
            if bytes_take == 0 && pkt_line.is_empty() {
                return Ok(body_bytes);
            }
            let command = self.parse_ref_update(&mut pkt_line);
            self.parse_capabilities(&String::from_utf8(pkt_line.to_vec()).unwrap());
            tracing::debug!("init comamnd: {:?}, caps:{:?}", command, self.capabilities);
            self.command_list.push(command);
            Ok(body_bytes.split_off(4))
        }
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
    pub fn parse_ref_update(&self, pkt_line: &mut Bytes) -> RefCommand {
        RefCommand::new(
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
        )
    }
}

pub async fn save_packfile(
    // &self,
    storage: Arc<dyn ObjectStorage>,
    object_map: ObjDecodedMap,
    repo_path: &Path,
) -> Result<(), anyhow::Error> {
    let nodes = build_node_tree(&object_map, repo_path).await.unwrap();
    storage.save_nodes(nodes).await.unwrap();

    let mut save_models: Vec<commit::ActiveModel> = Vec::new();
    for commit in &object_map.commits {
        save_models.push(commit.convert_to_model(repo_path));
    }

    storage.save_commits(save_models).await.unwrap();
    Ok(())
}

pub async fn handle_refs(storage: Arc<dyn ObjectStorage>, command: &RefCommand, path: &Path) {
    match command.command_type {
        CommandType::Create => {
            storage
                .save_refs(vec![command.convert_to_model(path.to_str().unwrap())])
                .await
        }
        CommandType::Delete => storage.delete_refs(command.old_id.clone(), path).await,
        CommandType::Update => {
            storage
                .update_refs(command.old_id.clone(), command.new_id.clone(), path)
                .await
        }
    }
}

fn read_until_white_space(bytes: &mut Bytes) -> String {
    let mut buf = Vec::new();
    while bytes.has_remaining() {
        let c = bytes.get_u8();
        if c.is_ascii_whitespace() {
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

    use crate::protocol::{Capability, CommandType, PackProtocol, RefCommand};

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
        let result = mock.parse_ref_update(&mut bytes);

        let command = RefCommand {
            ref_name: String::from("refs/heads/master\0"),
            old_id: String::from("0000000000000000000000000000000000000000"),
            new_id: String::from("27dd8d4cf39f3868c6eee38b601bc9e9939304f5"),
            status: String::from("ok"),
            error_msg: String::new(),
            command_type: CommandType::Create,
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
