//! Launcher language. The launcher follows the phone's language when that is
//! English or Vietnamese, or the language picked in Settings.
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Vi,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Vi => "vi",
        }
    }
    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "vi" => Some(Self::Vi),
            _ => None,
        }
    }
    /// The language's own name, as its Settings button shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Vi => "Tiếng Việt",
        }
    }
}

static ACTIVE: AtomicU8 = AtomicU8::new(0);

/// The language the launcher draws in now.
pub fn active() -> Lang {
    if ACTIVE.load(Ordering::Relaxed) == 1 {
        Lang::Vi
    } else {
        Lang::En
    }
}

pub fn set_active(lang: Lang) {
    ACTIVE.store(u8::from(lang == Lang::Vi), Ordering::Relaxed);
}

/// The phone's language as Java reports it; English when it is unknown.
pub fn system_lang() -> Lang {
    let code = crate::platform::with_context(|env, _activity| {
        let locale_class = env.find_class("java/util/Locale")?;
        let locale = env
            .call_static_method(
                locale_class,
                "getDefault",
                "()Ljava/util/Locale;",
                &[],
            )?
            .l()?;
        let code = env
            .call_method(&locale, "getLanguage", "()Ljava/lang/String;", &[])?
            .l()?;
        let code: String = env
            .get_string(&jni::objects::JString::from(code))?
            .into();
        Ok(code)
    });
    code.as_deref()
        .and_then(Lang::from_code)
        .unwrap_or_default()
}

/// The launcher's text for an English string, in the active language.
/// Strings without a translation stay in English.
pub fn tr(english: &'static str) -> &'static str {
    if active() == Lang::En {
        return english;
    }
    match english {
        "Play" => "Chơi",
        "Mods" => "Mod",
        "Resource Pack" => "Gói tài nguyên",
        "Shaders" => "Shader",
        "Installed" => "Đã cài",
        "Settings" => "Cài đặt",
        "Skins" => "Skin",
        "Show FPS In Game" => "Hiện FPS trong game",
        "Shows the frames-per-second counter over the game screen" => {
            "Hiện bộ đếm khung hình trên màn hình game"
        }
        "Follow phone language" => "Theo ngôn ngữ điện thoại",
        "Use the phone's language when it is English or Vietnamese" => {
            "Dùng ngôn ngữ điện thoại khi là tiếng Anh hoặc tiếng Việt"
        }
        "Language" => "Ngôn ngữ",
        "Used when following the phone language is off" => {
            "Dùng khi không theo ngôn ngữ điện thoại"
        }
        "Launch Minecraft" => "Khởi chạy Minecraft",
        "Install and manage mods" => "Cài và quản lý mod",
        "Textures and sounds" => "Texture và âm thanh",
        "Lighting and visual effects" => "Ánh sáng và hiệu ứng hình ảnh",
        "Versions and profiles on this device" => "Phiên bản và hồ sơ trên máy này",
        "Launcher and game options" => "Tùy chọn launcher và game",
        "Skins for your account" => "Skin cho tài khoản",
        _ => english,
    }
}
