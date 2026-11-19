use super::Origin;
use crate::model::{AiConfig, Settings, Translation, TranslationRequest};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PopupError {
    Settings,
    Endpoint,
    Model,
    Key,
    Network,
    Timeout,
    Incomplete,
    StreamUnsupported,
    Service,
}

impl PopupError {
    fn from_service(reason: &str) -> Self {
        match reason {
            crate::ai::INCOMPLETE => Self::Incomplete,
            crate::ai::STREAM_UNSUPPORTED => Self::StreamUnsupported,
            "请填写 AI 模型名称" => Self::Model,
            "官方 OpenAI 服务需要 API Key" => Self::Key,
            s if s.contains("服务地址") => Self::Endpoint,
            s if s.contains("请求超时") => Self::Timeout,
            s if s.contains("无法连接") => Self::Network,
            s if s.contains("HTTP 401") || s.contains("HTTP 403") => Self::Key,
            _ => Self::Service,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Phase {
    Preparing,
    Loading,
    Streaming {
        translation: String,
    },
    Success {
        translation: String,
        history_saved: bool,
    },
    Error {
        error: PopupError,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Selection {
    pub selection_id: u64,
    pub request_id: u64,
    pub locale: String,
    pub source_text: String,
    pub source_lang: String,
    pub target_lang: String,
    #[serde(flatten)]
    pub phase: Phase,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub version: u64,
    pub dismiss_request_id: Option<u64>,
    pub preparing: Option<Selection>,
    pub pinned: bool,
    pub selection: Option<Selection>,
}

struct Prepared {
    selection: Selection,
    origin: Origin,
    config: Result<AiConfig, PopupError>,
}

pub(super) struct Attempt {
    pub id: u64,
    pub request: TranslationRequest,
    pub config: Result<AiConfig, PopupError>,
}

impl Selection {
    fn attempt(&self, config: Result<AiConfig, PopupError>) -> Attempt {
        Attempt {
            id: self.request_id,
            request: TranslationRequest {
                text: self.source_text.clone(),
                source_lang: self.source_lang.clone(),
                target_lang: self.target_lang.clone(),
            },
            config,
        }
    }
}

#[derive(Default)]
pub(super) struct Session {
    version: u64,
    request_id: Option<u64>,
    pending: Option<Prepared>,
    current: Option<Selection>,
    pub pinned: bool,
    pub focused: bool,
}

impl Session {
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: self.version,
            dismiss_request_id: self.request_id,
            pinned: self.pinned,
            // 已固定的译文和候选选区分别绘制，来源复核失败时仍可对照上一段。
            preparing: self.pending.as_ref().map(|p| p.selection.clone()),
            selection: self.current.clone(),
        }
    }

    // 先交付复制结果并释放快捷键互斥，读取 SQLite 的耗时独立于复制时限。
    pub fn reserve(&mut self) -> u64 {
        self.version += 1;
        self.pending = None;
        self.request_id = Some(self.version);
        self.version
    }

    pub fn prepare_reserved(
        &mut self,
        id: u64,
        text: String,
        origin: Origin,
        config: Result<(Settings, AiConfig), String>,
    ) -> bool {
        if !self.matches_request(id) || self.pending.is_some() || self.active(id).is_some() {
            return false;
        }
        self.version += 1;
        let (settings, config) = match config {
            Ok((settings, config)) => (settings, Ok(config)),
            Err(_) => (Settings::default(), Err(PopupError::Settings)),
        };
        self.pending = Some(Prepared {
            selection: Selection {
                selection_id: id,
                request_id: id,
                locale: settings.locale,
                source_text: text,
                source_lang: settings.default_source_lang,
                target_lang: settings.default_target_lang,
                phase: Phase::Preparing,
            },
            config,
            origin,
        });
        true
    }

    pub fn origin(&self, id: u64) -> Option<Origin> {
        self.pending
            .as_ref()
            .filter(|p| p.selection.request_id == id)
            .map(|p| p.origin.clone())
    }

    pub fn abandon(&mut self, id: u64) -> bool {
        if self.matches_request(id) && self.active(id).is_none() {
            self.pending = None;
            self.request_id = self.current.as_ref().map(|s| s.request_id);
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn present(&mut self, id: u64) -> Option<Attempt> {
        self.origin(id)?;
        let mut pending = self.pending.take()?;
        pending.selection.phase = Phase::Loading;
        let attempt = pending.selection.attempt(pending.config);
        self.current = Some(pending.selection);
        self.focused = true;
        self.version += 1;
        Some(attempt)
    }

    pub fn retranslate(
        &mut self,
        id: u64,
        request: TranslationRequest,
        config: Result<AiConfig, String>,
    ) -> Result<Option<Attempt>, String> {
        if !self.matches_request(id) || request.text.trim().is_empty() {
            return Ok(None);
        }
        let Some(current) = self.current.as_mut().filter(|s| s.request_id == id) else {
            return Ok(None);
        };
        crate::validation::validate_languages(&request.source_lang, &request.target_lang)?;
        // 同一份输入在途时去重。成功或失败后，用户可以主动再次提交相同原文。
        if matches!(current.phase, Phase::Loading | Phase::Streaming { .. })
            && current.source_text == request.text
            && current.source_lang == request.source_lang
            && current.target_lang == request.target_lang
        {
            return Ok(None);
        }
        self.version += 1;
        current.source_text = request.text;
        current.source_lang = request.source_lang;
        current.target_lang = request.target_lang;
        current.request_id = self.version;
        self.request_id = Some(self.version);
        current.phase = Phase::Loading;
        Ok(Some(
            current.attempt(config.map_err(|_| PopupError::Settings)),
        ))
    }

    fn in_flight(&mut self, id: u64) -> Option<&mut Selection> {
        self.current.as_mut().filter(|s| {
            s.request_id == id && matches!(s.phase, Phase::Loading | Phase::Streaming { .. })
        })
    }

    pub fn append(&mut self, id: u64, piece: &str) -> bool {
        let Some(current) = self.in_flight(id) else {
            return false;
        };
        if let Phase::Streaming { translation } = &mut current.phase {
            translation.push_str(piece);
        } else {
            current.phase = Phase::Streaming {
                translation: piece.to_owned(),
            };
        }
        self.version += 1;
        true
    }

    pub fn finish(&mut self, id: u64, result: Result<Translation, PopupError>) -> bool {
        let Some(current) = self.in_flight(id) else {
            return false;
        };
        current.phase = match result {
            Ok(value) => Phase::Success {
                translation: value.translated_text,
                history_saved: value.warning.is_none(),
            },
            Err(error) => Phase::Error { error },
        };
        self.version += 1;
        true
    }

    pub fn active(&self, id: u64) -> Option<&Selection> {
        self.current
            .as_ref()
            .filter(|s| s.request_id == id && self.matches_request(id))
    }

    pub fn copied_text(&self, id: u64) -> Option<&str> {
        match &self.active(id)?.phase {
            Phase::Success { translation, .. } => Some(translation),
            _ => None,
        }
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.pinned = pinned;
        self.version += 1;
    }

    pub fn pending_id(&self) -> Option<u64> {
        self.pending.as_ref().map(|p| p.selection.request_id)
    }

    pub fn matches_request(&self, id: u64) -> bool {
        self.request_id == Some(id)
    }

    pub fn visible(&self) -> bool {
        self.current.is_some()
    }

    pub fn focus_changed(&mut self, focused: bool) -> bool {
        if !self.visible() {
            return false;
        }
        let dismiss = !focused && self.focused && !self.pinned;
        self.focused = focused;
        dismiss
    }

    pub fn clear(&mut self) {
        self.request_id = None;
        self.pending = None;
        self.current = None;
        self.focused = false;
        self.pinned = false;
        self.version += 1;
    }
}

impl Attempt {
    pub async fn run(
        self,
        store: &crate::storage::Store,
        ai: &crate::ai::AiService,
        on_delta: impl FnMut(&str) + Send,
    ) -> Result<Translation, PopupError> {
        crate::application::translate_with_config(store, ai, &self.request, &self.config?, on_delta)
            .await
            .map_err(|reason| PopupError::from_service(&reason))
    }
}
