#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ApiState<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    En,
    Zh,
}

impl Language {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Zh => "中文",
        }
    }

    pub(crate) fn document_code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh-CN",
        }
    }
}
