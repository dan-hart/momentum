// SPDX-License-Identifier: GPL-3.0-or-later
//! LibreSync v0.6 exports the entire State, even for engine.sync(adapter). Use an
//! isolated optional channel so a task-only peer never receives settings values.
use super::*;
const APP: &str = "io.github.dan_hart.Momentum.settings.v2";
const ADAPTER: &str = "portable-settings-v2";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct SettingsCapability {
    pub device_id: String,
    pub version: u32,
    pub port: u16,
}
struct Trust {
    base: Arc<Handler>,
    settings: Arc<dyn SettingsHandler>,
}
impl DeviceHandler for Trust {
    fn app_id(&self) -> &str {
        APP
    }
    fn is_linked(&self, i: &Identity) -> bool {
        self.settings.enabled() && self.base.is_linked(i)
    }
    fn is_linked_with_fingerprint(&self, i: &Identity, f: &str) -> bool {
        self.settings.enabled() && self.base.is_linked_with_fingerprint(i, f)
    }
    fn approve_link(&self, _i: &Identity) -> Result<bool> {
        Ok(false)
    }
    fn app_key(&self) -> Result<AppKey> {
        self.base.app_key()
    }
    fn set_app_key(&self, _k: &AppKey) -> Result<()> {
        Err(Error::Protocol("settings channel cannot replace pairing keys".into()))
    }
    fn device_keys(&self) -> Result<DeviceKeys> {
        self.base.device_keys()
    }
    fn set_device_keys(&self, _k: &DeviceKeys) -> Result<()> {
        Err(Error::Protocol(
            "settings channel cannot replace device identity".into(),
        ))
    }
}
struct Adapter {
    device_id: String,
    handler: Arc<dyn SettingsHandler>,
    active: Arc<std::sync::atomic::AtomicBool>,
}
impl LogicalAdapter for Adapter {
    fn id(&self) -> &str {
        ADAPTER
    }
    fn namespace(&self) -> &str {
        APP
    }
    fn load_records(&self, records: &mut RecordState) -> Result<()> {
        if !self.active.load(std::sync::atomic::Ordering::SeqCst) || !self.handler.enabled() {
            return Err(Error::Protocol("settings sync is disabled".into()));
        }
        let payload = self.handler.outgoing()?;
        if !records
            .get("portable", "SettingsV2", &self.device_id)?
            .is_some_and(|r| field_str(&r, "payload") == Some(payload.as_str()))
        {
            records.set(SyncRecord {
                schema: "portable".into(),
                entity: "SettingsV2".into(),
                id: self.device_id.clone(),
                fields: std::collections::BTreeMap::from([("payload".into(), FieldValue::String(payload))]),
                ..Default::default()
            })?;
        }
        Ok(())
    }
    fn apply_records(&self, view: &RecordView) -> Result<()> {
        if !self.active.load(std::sync::atomic::Ordering::SeqCst) || !self.handler.enabled() {
            return Err(Error::Protocol("settings sync is disabled".into()));
        }
        for record in view.snapshot()? {
            if record.tombstone || record.id == self.device_id {
                continue;
            }
            if record.schema != "portable" || record.entity != "SettingsV2" {
                return Err(Error::Protocol("unsupported settings channel version".into()));
            }
            let payload = field_str(&record, "payload")
                .ok_or_else(|| Error::Protocol("invalid settings channel payload".into()))?;
            self.handler.receive(&record.id, payload)?;
        }
        Ok(())
    }
}
/// Keep each peer's document scoped to that exchange. The core already retains
/// the merged causal values, so caching/echoing foreign documents in transport
/// state can misidentify a bootstrap review as a different linked device.
struct ChannelAdapter(Adapter);
impl libresync::DataAdapter for ChannelAdapter {
    fn id(&self) -> &str {
        ADAPTER
    }
    fn kind(&self) -> libresync::AdapterKind {
        libresync::AdapterKind::Logical
    }
    fn capabilities(&self) -> libresync::AdapterCapabilities {
        libresync::AdapterCapabilities::logical_snapshot()
    }
    fn load_into_state(&self, state: &mut State) -> Result<()> {
        // A bootstrap decision requires the peer's current image even if it has
        // not changed since the review. Fresh, small snapshots also prevent a
        // third peer's cached document from becoming this connection's review.
        *state = State::new(&self.0.device_id);
        self.0.load_records(&mut RecordState::new(state, APP))
    }
    fn apply_from_state(&self, _state: &State) -> Result<()> {
        Ok(())
    }
    fn owns_key(&self, _key: &str) -> bool {
        true
    }
    fn apply_entries(&self, _state: &mut State, entries: Vec<libresync::Entry>) -> Result<usize> {
        let count = entries.len();
        let mut incoming = State::new("settings-incoming");
        for entry in &entries {
            if libresync::entry_to_record(entry, APP)?.is_none() {
                return Err(Error::Protocol("unsupported settings channel namespace".into()));
            }
        }
        incoming.merge_snapshot(entries);
        self.0.apply_records(&RecordView::new(&incoming, APP))?;
        Ok(count)
    }
}
pub(super) struct SettingsChannel {
    engine: Mutex<Engine>,
    cancel: libresync::CancelToken,
    pub port: u16,
    handler: Arc<dyn SettingsHandler>,
    active: Arc<std::sync::atomic::AtomicBool>,
}
impl SettingsChannel {
    fn start(identity: &Identity, base: Arc<Handler>, handler: Arc<dyn SettingsHandler>) -> Result<Arc<Self>> {
        let mut engine = Engine::new(
            EngineConfig::new(Identity::new(&identity.device_id, APP, &identity.user_id))
                .with_listen_addr(([0, 0, 0, 0], 0).into()),
            State::new(&identity.device_id),
            Arc::new(Trust {
                base,
                settings: handler.clone(),
            }),
        );
        let active = Arc::new(std::sync::atomic::AtomicBool::new(true));
        engine.register_adapter(Arc::new(ChannelAdapter(Adapter {
            device_id: identity.device_id.clone(),
            handler: handler.clone(),
            active: active.clone(),
        })))?;
        let addr = engine.start_listening()?;
        Ok(Arc::new(Self {
            engine: Mutex::new(engine),
            cancel: libresync::CancelToken::new(),
            port: addr.port(),
            handler,
            active,
        }))
    }
    fn sync(&self, device: &LinkedDevice, cap: &SettingsCapability) -> Result<()> {
        if !self.handler.enabled() {
            return Err(Error::Protocol("settings sync is disabled".into()));
        }
        if cap.version != SETTINGS_CHANNEL_VERSION {
            return Err(Error::Protocol(
                "unsupported settings channel version on the linked device".into(),
            ));
        }
        if cap.port == 0 {
            return Err(Error::Protocol("settings sync is disabled on the linked device".into()));
        }
        let mut address = device
            .address
            .ok_or_else(|| Error::Protocol("linked device has no address".into()))?;
        address.set_port(cap.port);
        self.engine
            .lock()
            .map_err(|_| Error::Protocol("settings channel poisoned".into()))?
            .sync(
                &SyncRequest::new(address, ADAPTER)
                    .with_expected_device_id(&device.device_id)
                    .with_expected_fingerprint(&device.fingerprint)
                    .with_cancel(self.cancel.clone()),
            )?;
        Ok(())
    }
    pub(super) fn stop(&self) {
        self.active.store(false, std::sync::atomic::Ordering::SeqCst);
        self.cancel.cancel();
        if let Ok(mut engine) = self.engine.lock() {
            let _ = engine.stop_listening();
        }
    }
}
impl Drop for SettingsChannel {
    fn drop(&mut self) {
        self.stop();
    }
}
fn capability(journal: &Arc<Mutex<Journal>>, device: &LinkedDevice) -> Result<SettingsCapability> {
    journal
        .lock()
        .map_err(|_| Error::Protocol("journal poisoned".into()))?
        .settings_peers
        .get(&device.device_id)
        .cloned()
        .ok_or_else(|| {
            Error::Protocol("settings sync is unsupported by this linked device; update Momentum on that device".into())
        })
}
pub(super) fn sync_optional(
    channel: &Arc<Mutex<Option<Arc<SettingsChannel>>>>,
    handler: &Arc<Mutex<Option<Arc<dyn SettingsHandler>>>>,
    journal: &Arc<Mutex<Journal>>,
    device: &LinkedDevice,
) {
    let callback = handler.lock().ok().and_then(|h| h.clone());
    let Some(callback) = callback.filter(|h| h.enabled()) else {
        return;
    };
    let result = capability(journal, device).and_then(|cap| {
        let channel = channel
            .lock()
            .map_err(|_| Error::Protocol("settings channel poisoned".into()))?
            .clone()
            .ok_or_else(|| Error::Protocol("settings channel not running".into()))?;
        channel.sync(device, &cap)
    });
    if let Err(error) = result {
        callback.failed(error.to_string());
    }
}
impl P2p {
    pub fn configure_settings(&self, handler: Arc<dyn SettingsHandler>) -> Result<bool> {
        *self
            .settings_handler
            .lock()
            .map_err(|_| Error::Protocol("settings handler poisoned".into()))? = Some(handler.clone());
        let mut channel = self
            .settings_channel
            .lock()
            .map_err(|_| Error::Protocol("settings channel poisoned".into()))?;
        if handler.enabled() {
            if channel.is_none() {
                *channel = Some(SettingsChannel::start(&self.identity, self.handler.clone(), handler)?);
            }
        } else if let Some(old) = channel.take() {
            old.stop();
        }
        let cap = SettingsCapability {
            device_id: self.identity.device_id.clone(),
            version: SETTINGS_CHANNEL_VERSION,
            port: channel.as_ref().map(|c| c.port).unwrap_or(0),
        };
        let mut j = self.journal();
        let changed = j.settings_own.as_ref() != Some(&cap);
        if changed {
            let mut candidate = j.clone();
            candidate.settings_own = Some(cap);
            candidate.save(&self.journal_path).map_err(io)?;
            *j = candidate;
        }
        Ok(changed)
    }
    pub fn sync_settings_with(&self, device: &LinkedDevice) -> Result<()> {
        let cap = capability(&self.journal, device)?;
        let channel = self
            .settings_channel
            .lock()
            .map_err(|_| Error::Protocol("settings channel poisoned".into()))?
            .clone()
            .ok_or_else(|| Error::Protocol("settings sync is disabled".into()))?;
        let r = channel.sync(device, &cap);
        if let Err(error) = &r {
            channel.handler.failed(error.to_string());
        }
        r
    }
}
