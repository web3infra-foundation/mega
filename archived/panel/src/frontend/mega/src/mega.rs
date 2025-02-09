use crate::mega_settings::MegaSettings;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient};
use gpui::{hash, AppContext, EventEmitter, ModelContext};
use radix_trie::{Trie, TrieCommon};
use reqwest_client::ReqwestClient;
use settings::Settings;
use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use utils::api::{
    ConfigRequest, ConfigResponse, MountRequest, MountResponse, MountsResponse, UmountRequest,
    UmountResponse,
};

mod mega_settings;
pub mod utils;

pub fn init(cx: &mut AppContext) {
    Mega::init(cx);
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    FuseRunning(bool),
    FuseMounted(Option<PathBuf>),
    FuseCheckout(Option<PathBuf>),
}

#[allow(unused)]
struct CheckoutState {
    path: PathBuf,
    mounted: bool,
    notify: bool,
}

pub struct Mega {
    fuse_executable: PathBuf,

    fuse_running: bool,
    fuse_mounted: bool,
    heartbeat: bool,

    mount_point: Option<PathBuf>,
    checkout_lut: Trie<PathBuf, u64>,
    checkout_path: BTreeSet<u64>,

    mega_url: String,
    fuse_url: String,
    http_client: Arc<ReqwestClient>,
}

pub struct MegaFuse {}

impl EventEmitter<Event> for Mega {}

impl Debug for Mega {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lut = self
            .checkout_lut
            .keys()
            .map(|key| key.to_path_buf())
            .collect::<Vec<PathBuf>>();

        write!(
            f,
            "fuse_executable: {:?}, mega_url: {}, fuse_url: {}\n\
            LUT: {:?}",
            self.fuse_executable, self.mega_url, self.fuse_url, lut
        )
    }
}

impl Mega {
    pub fn init_settings(cx: &mut AppContext) {
        MegaSettings::register(cx);
    }

    pub fn init(cx: &mut AppContext) {
        Self::init_settings(cx);
    }

    pub fn new(cx: &mut AppContext) -> Self {
        let mount_path = MegaSettings::get_global(cx).mount_point.clone();
        let mega_url = MegaSettings::get_global(cx).mega_url.clone();
        let fuse_url = MegaSettings::get_global(cx).fuse_url.clone();
        let fuse_executable = MegaSettings::get_global(cx).fuse_executable.clone();

        // To not affected by global proxy settings.
        let client = ReqwestClient::new();

        println!("Mount point: {mount_path:?}");

        let mount_point = if mount_path.exists() {
            Some(mount_path)
        } else {
            log::error!("Mount point in setting does not exist");
            None
        };

        Mega {
            fuse_executable,

            fuse_running: false,
            fuse_mounted: false,
            heartbeat: false,

            mount_point,
            checkout_lut: Trie::default(),
            checkout_path: Default::default(),

            mega_url,
            fuse_url,
            http_client: Arc::new(client),
        }
    }

    pub fn update_status(&mut self, cx: &mut ModelContext<Self>) {
        let config = self.get_fuse_config(cx);
        let checkouts = self.get_checkout_paths(cx);

        cx.spawn(|this, mut cx| async move {
            // When mount point changed, emit an event.
            // update mount point if it's none.
            if let Ok(opt) = config.await {
                match opt {
                    None => {
                        // This means we cannot connect to a localhost port.
                        // So we can assume that fuse has been dead.
                        let _ = this.update(&mut cx, |mega, cx| {
                            mega.fuse_running = false;
                            mega.fuse_mounted = false;
                            cx.emit(Event::FuseRunning(false));
                            cx.emit(Event::FuseMounted(None));
                            return;
                        });
                    }

                    Some(config) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            let path = PathBuf::from(config.config.mount_path);
                            if (this.fuse_mounted && this.fuse_running)
                                && this.mount_point.is_some()
                            {
                                if let Some(inner) = &this.mount_point {
                                    if !inner.eq(&path) {
                                        this.mount_point = Some(path);
                                        // cx.emit(Event::FuseMounted(this.mount_point.clone()));
                                    }
                                }
                            } else if this.mount_point.is_none() {
                                this.mount_point = Some(path);
                                if this.fuse_running {
                                    this.fuse_mounted = true;
                                    cx.emit(Event::FuseMounted(this.mount_point.clone()));
                                }
                            }
                        });
                    }
                }
            }

            if let Ok(Some(info)) = checkouts.await {
                // Check if checkout-ed paths are correct
                let _ = this.update(&mut cx, |mega, _cx| {
                    mega.fuse_running = true;

                    let trie = &mut mega.checkout_lut;
                    for i in info.mounts.iter() {
                        let path = i.path.parse().unwrap();

                        let missing = trie.get_ancestor(&path).is_none();
                        if missing {
                            // Should not happen unless on startup.
                            mega.checkout_path.insert(hash(&path));
                            trie.insert(path, i.inode);
                            // cx.emit(Event::FuseCheckout(Some(PathBuf::from(i.path.clone()))));
                        }
                    }
                });
            }
        })
        .detach();
    }

    pub fn status(&self) -> (bool, bool) {
        (self.fuse_running, self.fuse_mounted)
    }

    pub fn checkout_points(&self) -> Vec<String> {
        self.checkout_lut
            .keys()
            .map(|key| key.as_path().to_str().unwrap().to_string())
            .collect()
    }

    /// ## Toggle Fuse checkouts
    /// Checkout or un-checkout the paths in zed.
    /// Does nothing if fuse not running.
    pub fn toggle_fuse(&mut self, cx: &mut ModelContext<Self>) {
        self.update_status(cx);
        let paths = &self.checkout_lut;

        if !self.fuse_running {
            return;
        }

        if !self.fuse_mounted {
            for (_, (p, _)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega
                        .update(&mut cx, |this, cx| this.checkout_path(cx, path))
                        .expect("mega delegate not be dropped");

                    if let Ok(Some(_resp)) = recv.await {
                        // mega.update(&mut cx, |_, cx| {
                        //     let buf = PathBuf::from(resp.mount.path.clone());
                        //     cx.emit(Event::FuseCheckout(Some(buf)));
                        // })
                    }
                })
                .detach();
            }

            self.fuse_mounted = true;
            cx.emit(Event::FuseMounted(self.mount_point.clone()));
        } else {
            for (_, (p, _)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega
                        .update(&mut cx, |this, cx| this.restore_path(cx, &path))
                        .expect("mega delegate not be dropped");

                    if let Ok(Some(_resp)) = recv.await {
                        // mega.update(&mut cx, |_, cx| {
                        //     // TODO use a new check out state struct
                        //     cx.emit(Event::FuseCheckout(None));
                        // })
                    }
                })
                .detach();
            }
        }
    }

    /// ## Toggle Fuse Mount
    /// In fact, we cannot `mount` or `umount` a fuse from zed.
    ///
    /// This function only opens up a new scorpio executable if it detects fuse not running.
    pub fn toggle_mount(&mut self, cx: &mut ModelContext<Self>) {
        if !self.fuse_running {
            if let Ok(_) =  Command::new(&self.fuse_executable).spawn() {
                self.update_status(cx);
            } else {
                log::error!("Cannot start up fuse, check your settings");
            }
        }
    }

    pub fn checkout_path(
        &self,
        cx: &ModelContext<Self>,
        path: PathBuf,
    ) -> Receiver<Option<MountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/fs/mount", base = self.fuse_url);

        // If it panics, that means there's a bug in code.
        let path = path.to_str().unwrap();
        let req = MountRequest { path };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), AsyncBody::from(body)).await {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<MountResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn restore_path(
        &self,
        cx: &ModelContext<Self>,
        path: &PathBuf,
    ) -> Receiver<Option<UmountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/fs/umount", base = self.fuse_url);

        // If panics here, that means there's a bug in code.
        let inode = self.checkout_lut.get_ancestor_value(path);
        let req = UmountRequest {
            path: Some(path.to_str().unwrap()),
            inode: Some(inode.unwrap().to_owned()),
        };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), AsyncBody::from(body)).await {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<UmountResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn get_checkout_paths(&self, cx: &ModelContext<Self>) -> Receiver<Option<MountsResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/fs/mpoint", base = self.fuse_url);

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.get(uri.as_str(), AsyncBody::empty(), false).await {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<MountsResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn get_fuse_config(&self, cx: &ModelContext<Self>) -> Receiver<Option<ConfigResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/config", base = self.fuse_url);

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.get(uri.as_str(), AsyncBody::empty(), false).await {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<ConfigResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn set_fuse_config(&self, cx: &mut ModelContext<Self>) -> Receiver<Option<ConfigResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/config", base = self.fuse_url);
        let config = ConfigRequest {
            mega_url: None,
            mount_path: None,
            store_path: None,
        };

        let config = serde_json::to_string(&config).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), config.into()).await {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<ConfigResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn heartbeat(&mut self, cx: &mut ModelContext<Self>) {
        if self.heartbeat {
            return;
        } else {
            self.heartbeat = true;
        }

        cx.spawn(|this, mut cx| async move {
            loop {
                this.update(&mut cx, |mega, cx| {
                    mega.update_status(cx);
                })
                .expect("mega delegate not be dropped");

                let dur = Duration::from_secs(30);
                cx.background_executor().timer(dur).await;
            }
        })
        .detach();
    }

    pub fn is_path_checkout(&self, path: &PathBuf) -> bool {
        self.checkout_lut.get_ancestor(path).is_some()
    }

    pub fn is_path_checkout_root(&self, path: &PathBuf) -> bool {
        self.checkout_path.contains(&hash(path))
    }

    pub fn mark_checkout(&mut self, cx: &mut ModelContext<Self>, path: PathBuf, inode: u64) {
        cx.emit(Event::FuseCheckout(Some(path.clone())));
        self.checkout_path.insert(hash(&path));
        self.checkout_lut.insert(path, inode);
    }

    pub fn mark_commited(&mut self, cx: &mut ModelContext<Self>, path: &PathBuf) {
        cx.emit(Event::FuseCheckout(None));
        self.checkout_path.remove(&hash(path));
        self.checkout_lut.remove(path);
    }

    pub fn mount_point(&self) -> Option<&PathBuf> {
        match self.mount_point {
            Some(ref path) => Some(path),
            None => None,
        }
    }
}
