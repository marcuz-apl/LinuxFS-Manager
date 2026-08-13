//! Bundled, left-to-right UI copy for the desktop application.

pub const AUTOMATIC_LANGUAGE: &str = "auto";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiLanguage {
    English,
    French,
    German,
    Spanish,
    PortugueseBrazil,
    Italian,
    Polish,
    Russian,
    ChineseSimplified,
    ChineseTraditional,
    Japanese,
    Korean,
}

impl UiLanguage {
    pub const ALL: [Self; 12] = [
        Self::English,
        Self::French,
        Self::German,
        Self::Spanish,
        Self::PortugueseBrazil,
        Self::Italian,
        Self::Polish,
        Self::Russian,
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::Japanese,
        Self::Korean,
    ];

    pub const fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr-FR",
            Self::German => "de-DE",
            Self::Spanish => "es-ES",
            Self::PortugueseBrazil => "pt-BR",
            Self::Italian => "it-IT",
            Self::Polish => "pl-PL",
            Self::Russian => "ru-RU",
            Self::ChineseSimplified => "zh-CN",
            Self::ChineseTraditional => "zh-TW",
            Self::Japanese => "ja-JP",
            Self::Korean => "ko-KR",
        }
    }

    pub const fn self_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Spanish => "Español",
            Self::PortugueseBrazil => "Português (Brasil)",
            Self::Italian => "Italiano",
            Self::Polish => "Polski",
            Self::Russian => "Русский",
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
        }
    }

    pub const fn selector_index(self) -> i32 {
        match self {
            Self::English => 1,
            Self::French => 2,
            Self::German => 3,
            Self::Spanish => 4,
            Self::PortugueseBrazil => 5,
            Self::Italian => 6,
            Self::Polish => 7,
            Self::Russian => 8,
            Self::ChineseSimplified => 9,
            Self::ChineseTraditional => 10,
            Self::Japanese => 11,
            Self::Korean => 12,
        }
    }

    pub const fn default_font_family(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "Microsoft YaHei UI",
            Self::ChineseTraditional => "Microsoft JhengHei UI",
            Self::Japanese => "Yu Gothic UI",
            Self::Korean => "Malgun Gothic",
            _ => "Segoe UI",
        }
    }
}

pub fn language_from_selector(index: i32) -> Option<UiLanguage> {
    UiLanguage::ALL
        .into_iter()
        .find(|language| language.selector_index() == index)
}

pub fn language_from_self_name(name: &str) -> Option<UiLanguage> {
    UiLanguage::ALL
        .into_iter()
        .find(|language| language.self_name() == name)
}

pub fn resolve_language(preference: Option<&str>, windows_locale: &str) -> UiLanguage {
    preference
        .and_then(parse_supported_language)
        .or_else(|| parse_supported_language(windows_locale))
        .unwrap_or(UiLanguage::English)
}

fn parse_supported_language(value: &str) -> Option<UiLanguage> {
    let normalized = value.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.is_empty() || normalized == AUTOMATIC_LANGUAGE {
        return None;
    }
    let exact = match normalized.as_str() {
        "en" | "en-us" | "en-gb" => Some(UiLanguage::English),
        "fr" | "fr-fr" => Some(UiLanguage::French),
        "de" | "de-de" => Some(UiLanguage::German),
        "es" | "es-es" => Some(UiLanguage::Spanish),
        "pt" | "pt-br" => Some(UiLanguage::PortugueseBrazil),
        "it" | "it-it" => Some(UiLanguage::Italian),
        "pl" | "pl-pl" => Some(UiLanguage::Polish),
        "ru" | "ru-ru" => Some(UiLanguage::Russian),
        "zh-cn" | "zh-hans" | "zh-hans-cn" => Some(UiLanguage::ChineseSimplified),
        "zh-tw" | "zh-hant" | "zh-hant-tw" => Some(UiLanguage::ChineseTraditional),
        "ja" | "ja-jp" => Some(UiLanguage::Japanese),
        "ko" | "ko-kr" => Some(UiLanguage::Korean),
        _ => None,
    };
    exact.or_else(|| match normalized.split('-').next() {
        Some("fr") => Some(UiLanguage::French),
        Some("de") => Some(UiLanguage::German),
        Some("es") => Some(UiLanguage::Spanish),
        Some("pt") => Some(UiLanguage::PortugueseBrazil),
        Some("it") => Some(UiLanguage::Italian),
        Some("pl") => Some(UiLanguage::Polish),
        Some("ru") => Some(UiLanguage::Russian),
        Some("ja") => Some(UiLanguage::Japanese),
        Some("ko") => Some(UiLanguage::Korean),
        _ => None,
    })
}

#[cfg(windows)]
#[allow(unsafe_code)]
pub fn windows_user_locale() -> String {
    use windows_sys::Win32::{
        Globalization::GetUserDefaultLocaleName, System::SystemServices::LOCALE_NAME_MAX_LENGTH,
    };

    let mut buffer = [0_u16; LOCALE_NAME_MAX_LENGTH as usize];
    // SAFETY: `buffer` is writable UTF-16 storage with the API-mandated capacity.
    let length =
        unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH as i32) };
    if length <= 1 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..length as usize - 1])
    }
}

#[cfg(not(windows))]
pub fn windows_user_locale() -> String {
    String::new()
}

#[derive(Clone, Copy, Debug)]
pub struct UiCopy {
    pub language: UiLanguage,
    pub app_title: &'static str,
    pub app_subtitle: &'static str,
    pub scan_drives: &'static str,
    pub open_image: &'static str,
    pub about: &'static str,
    pub sources: &'static str,
    pub sources_subtitle: &'static str,
    pub sources_empty: &'static str,
    pub filesystem_details: &'static str,
    pub open_filesystem_image: &'static str,
    pub image_placeholder: &'static str,
    pub mount: &'static str,
    pub unmount: &'static str,
    pub open_in_explorer: &'static str,
    pub details: &'static str,
    pub read_only_warning: &'static str,
    pub version: &'static str,
    pub about_description: &'static str,
    pub copyright: &'static str,
    pub close: &'static str,
    pub prerequisite_title: &'static str,
    pub prerequisite_subtitle: &'static str,
    pub to_continue: &'static str,
    pub prerequisite_step_one: &'static str,
    pub prerequisite_step_two: &'static str,
    pub prerequisite_step_three: &'static str,
    pub prerequisite_notice: &'static str,
    pub download_winfsp: &'static str,
    pub recheck: &'static str,
    pub automatic_language: &'static str,
}

impl UiCopy {
    fn new(language: UiLanguage) -> Self {
        let text = |key| text(language, key);
        Self {
            language,
            app_title: text(TextKey::AppTitle),
            app_subtitle: text(TextKey::AppSubtitle),
            scan_drives: text(TextKey::ScanDrives),
            open_image: text(TextKey::OpenImage),
            about: text(TextKey::About),
            sources: text(TextKey::Sources),
            sources_subtitle: text(TextKey::SourcesSubtitle),
            sources_empty: text(TextKey::SourcesEmpty),
            filesystem_details: text(TextKey::FilesystemDetails),
            open_filesystem_image: text(TextKey::OpenFilesystemImage),
            image_placeholder: text(TextKey::ImagePlaceholder),
            mount: text(TextKey::Mount),
            unmount: text(TextKey::Unmount),
            open_in_explorer: text(TextKey::OpenInExplorer),
            details: text(TextKey::Details),
            read_only_warning: text(TextKey::ReadOnlyWarning),
            version: text(TextKey::Version),
            about_description: text(TextKey::AboutDescription),
            copyright: text(TextKey::Copyright),
            close: text(TextKey::Close),
            prerequisite_title: text(TextKey::PrerequisiteTitle),
            prerequisite_subtitle: text(TextKey::PrerequisiteSubtitle),
            to_continue: text(TextKey::ToContinue),
            prerequisite_step_one: text(TextKey::PrerequisiteStepOne),
            prerequisite_step_two: text(TextKey::PrerequisiteStepTwo),
            prerequisite_step_three: text(TextKey::PrerequisiteStepThree),
            prerequisite_notice: text(TextKey::PrerequisiteNotice),
            download_winfsp: text(TextKey::DownloadWinFsp),
            recheck: text(TextKey::Recheck),
            automatic_language: text(TextKey::AutomaticLanguage),
        }
    }

    pub fn language_options(self) -> Vec<String> {
        std::iter::once(self.automatic_language.to_owned())
            .chain(
                UiLanguage::ALL
                    .into_iter()
                    .map(|language| language.self_name().to_owned()),
            )
            .collect()
    }

    pub fn ready(self) -> String {
        dynamic(self.language, DynamicKey::Ready, "")
    }
    pub fn refresh_failed(self, error: &str) -> String {
        dynamic(self.language, DynamicKey::RefreshFailed, error)
    }
    pub fn mounting(self) -> String {
        dynamic(self.language, DynamicKey::Mounting, "")
    }
    pub fn mounted(self, point: &str) -> String {
        dynamic(self.language, DynamicKey::Mounted, point)
    }
    pub fn unmounting(self) -> String {
        dynamic(self.language, DynamicKey::Unmounting, "")
    }
    pub fn unmounted(self) -> String {
        dynamic(self.language, DynamicKey::Unmounted, "")
    }
    pub fn mount_failed(self, error: &str) -> String {
        dynamic(self.language, DynamicKey::MountFailed, error)
    }
    pub fn unmount_failed(self, error: &str) -> String {
        dynamic(self.language, DynamicKey::UnmountFailed, error)
    }
    pub fn explorer_opened(self, point: &str) -> String {
        dynamic(self.language, DynamicKey::ExplorerOpened, point)
    }
    pub fn existing_mount_available(self, error: &str) -> String {
        dynamic(self.language, DynamicKey::ExistingMountAvailable, error)
    }
    pub fn language_save_failed(self, error: &str) -> String {
        let message = match self.language {
            UiLanguage::English => "Could not save language preference",
            UiLanguage::French => "Impossible d’enregistrer la préférence de langue",
            UiLanguage::German => "Spracheinstellung konnte nicht gespeichert werden",
            UiLanguage::Spanish => "No se pudo guardar la preferencia de idioma",
            UiLanguage::PortugueseBrazil => "Não foi possível salvar a preferência de idioma",
            UiLanguage::Italian => "Impossibile salvare la preferenza della lingua",
            UiLanguage::Polish => "Nie można zapisać preferencji języka",
            UiLanguage::Russian => "Не удалось сохранить настройку языка",
            UiLanguage::ChineseSimplified => "无法保存语言首选项",
            UiLanguage::ChineseTraditional => "無法儲存語言偏好設定",
            UiLanguage::Japanese => "言語設定を保存できませんでした",
            UiLanguage::Korean => "언어 기본 설정을 저장할 수 없습니다",
        };
        format!("{message}: {error}")
    }
    pub fn no_source_loaded(self) -> &'static str {
        match self.language {
            UiLanguage::English => "No source loaded",
            UiLanguage::French => "Aucune source chargée",
            UiLanguage::German => "Keine Quelle geladen",
            UiLanguage::Spanish => "No hay fuente cargada",
            UiLanguage::PortugueseBrazil => "Nenhuma fonte carregada",
            UiLanguage::Italian => "Nessuna origine caricata",
            UiLanguage::Polish => "Nie załadowano źródła",
            UiLanguage::Russian => "Источник не загружен",
            UiLanguage::ChineseSimplified => "未加载源",
            UiLanguage::ChineseTraditional => "未載入來源",
            UiLanguage::Japanese => "ソースが読み込まれていません",
            UiLanguage::Korean => "원본이 로드되지 않았습니다",
        }
    }
    pub fn open_raw_image_hint(self) -> &'static str {
        match self.language {
            UiLanguage::English => "Open a raw Linux filesystem image to inspect it.",
            UiLanguage::French => {
                "Ouvrez une image brute de système de fichiers Linux pour l’inspecter."
            }
            UiLanguage::German => "Öffnen Sie ein rohes Linux-Dateisystem-Image zur Untersuchung.",
            UiLanguage::Spanish => {
                "Abra una imagen sin procesar de un sistema de archivos Linux para inspeccionarla."
            }
            UiLanguage::PortugueseBrazil => {
                "Abra uma imagem bruta de sistema de arquivos Linux para examiná-la."
            }
            UiLanguage::Italian => "Apri un’immagine raw di filesystem Linux per esaminarla.",
            UiLanguage::Polish => "Otwórz surowy obraz systemu plików Linux, aby go sprawdzić.",
            UiLanguage::Russian => {
                "Откройте необработанный образ файловой системы Linux для просмотра."
            }
            UiLanguage::ChineseSimplified => "打开原始 Linux 文件系统映像以进行查看。",
            UiLanguage::ChineseTraditional => "開啟原始 Linux 檔案系統映像檔以進行檢視。",
            UiLanguage::Japanese => "raw Linux ファイルシステムイメージを開いて確認します。",
            UiLanguage::Korean => "원시 Linux 파일 시스템 이미지를 열어 검사하세요.",
        }
    }
    pub fn no_compatible_source(self) -> &'static str {
        match self.language {
            UiLanguage::English => "No compatible source",
            UiLanguage::French => "Aucune source compatible",
            UiLanguage::German => "Keine kompatible Quelle",
            UiLanguage::Spanish => "No hay fuente compatible",
            UiLanguage::PortugueseBrazil => "Nenhuma fonte compatível",
            UiLanguage::Italian => "Nessuna origine compatibile",
            UiLanguage::Polish => "Brak zgodnego źródła",
            UiLanguage::Russian => "Нет совместимого источника",
            UiLanguage::ChineseSimplified => "没有兼容的源",
            UiLanguage::ChineseTraditional => "沒有相容的來源",
            UiLanguage::Japanese => "互換性のあるソースがありません",
            UiLanguage::Korean => "호환되는 원본이 없습니다",
        }
    }
    pub fn physical_scan_empty_details(self) -> &'static str {
        match self.language {
            UiLanguage::English => {
                "No supported Linux filesystem was found, or Windows denied raw-disk access. Run elevated to scan physical disks."
            }
            UiLanguage::French => {
                "Aucun système de fichiers Linux pris en charge n’a été trouvé, ou Windows a refusé l’accès brut au disque. Exécutez en tant qu’administrateur pour analyser les disques physiques."
            }
            UiLanguage::German => {
                "Kein unterstütztes Linux-Dateisystem gefunden, oder Windows verweigerte den Rohdatenträgerzugriff. Führen Sie die App erhöht aus, um physische Datenträger zu scannen."
            }
            UiLanguage::Spanish => {
                "No se encontró un sistema de archivos Linux compatible o Windows denegó el acceso sin procesar al disco. Ejecute como administrador para analizar discos físicos."
            }
            UiLanguage::PortugueseBrazil => {
                "Nenhum sistema de arquivos Linux compatível foi encontrado ou o Windows negou o acesso bruto ao disco. Execute como administrador para verificar discos físicos."
            }
            UiLanguage::Italian => {
                "Non è stato trovato alcun filesystem Linux supportato oppure Windows ha negato l’accesso raw al disco. Esegui come amministratore per analizzare i dischi fisici."
            }
            UiLanguage::Polish => {
                "Nie znaleziono obsługiwanego systemu plików Linux albo Windows odmówił surowego dostępu do dysku. Uruchom jako administrator, aby skanować dyski fizyczne."
            }
            UiLanguage::Russian => {
                "Поддерживаемая файловая система Linux не найдена либо Windows запретила прямой доступ к диску. Запустите от имени администратора для сканирования физических дисков."
            }
            UiLanguage::ChineseSimplified => {
                "未找到受支持的 Linux 文件系统，或者 Windows 拒绝了原始磁盘访问。请以管理员身份运行以扫描物理磁盘。"
            }
            UiLanguage::ChineseTraditional => {
                "找不到支援的 Linux 檔案系統，或 Windows 拒絕原始磁碟存取。請以系統管理員身分執行以掃描實體磁碟。"
            }
            UiLanguage::Japanese => {
                "対応する Linux ファイルシステムが見つからないか、Windows が raw ディスクアクセスを拒否しました。物理ディスクをスキャンするには管理者として実行してください。"
            }
            UiLanguage::Korean => {
                "지원되는 Linux 파일 시스템을 찾을 수 없거나 Windows가 원시 디스크 액세스를 거부했습니다. 물리 디스크를 검색하려면 관리자 권한으로 실행하세요."
            }
        }
    }
    pub fn image_open_failed_details(self) -> &'static str {
        match self.language {
            UiLanguage::English => "The image could not be opened safely.",
            UiLanguage::French => "L’image n’a pas pu être ouverte en toute sécurité.",
            UiLanguage::German => "Das Image konnte nicht sicher geöffnet werden.",
            UiLanguage::Spanish => "La imagen no se pudo abrir de forma segura.",
            UiLanguage::PortugueseBrazil => "Não foi possível abrir a imagem com segurança.",
            UiLanguage::Italian => "Non è stato possibile aprire l’immagine in modo sicuro.",
            UiLanguage::Polish => "Nie można było bezpiecznie otworzyć obrazu.",
            UiLanguage::Russian => "Не удалось безопасно открыть образ.",
            UiLanguage::ChineseSimplified => "无法安全地打开映像。",
            UiLanguage::ChineseTraditional => "無法安全地開啟映像檔。",
            UiLanguage::Japanese => "イメージを安全に開けませんでした。",
            UiLanguage::Korean => "이미지를 안전하게 열 수 없습니다.",
        }
    }
    pub fn is_complete(self) -> bool {
        TextKey::ALL
            .into_iter()
            .all(|key| !text(self.language, key).trim().is_empty())
    }
}

pub fn catalog(language: UiLanguage) -> UiCopy {
    UiCopy::new(language)
}

#[derive(Clone, Debug)]
pub struct LocalizedCatalog {
    pub copy: UiCopy,
    overrides: std::collections::BTreeMap<String, String>,
}

impl LocalizedCatalog {
    pub fn text(&self, key: &str, fallback: &str) -> String {
        self.overrides
            .get(key)
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    }

    pub fn language_options(&self) -> Vec<String> {
        std::iter::once(self.text("automatic_language", self.copy.automatic_language))
            .chain(
                UiLanguage::ALL
                    .into_iter()
                    .map(|language| language.self_name().to_owned()),
            )
            .collect()
    }
}

pub fn load_catalog(language: UiLanguage, directory: &std::path::Path) -> LocalizedCatalog {
    let copy = catalog(language);
    let path = directory.join(format!("{}.toml", language.tag()));
    let overrides = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok())
        .and_then(|document| {
            (document.get("language").and_then(toml::Value::as_str) == Some(language.tag()))
                .then_some(document)
        })
        .and_then(|document| {
            document
                .get("strings")
                .and_then(toml::Value::as_table)
                .cloned()
        })
        .map(|strings| {
            strings
                .into_iter()
                .filter_map(|(key, value)| value.as_str().map(|value| (key, value.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    LocalizedCatalog { copy, overrides }
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum TextKey {
    AppTitle,
    AppSubtitle,
    ScanDrives,
    OpenImage,
    About,
    Sources,
    SourcesSubtitle,
    SourcesEmpty,
    FilesystemDetails,
    OpenFilesystemImage,
    ImagePlaceholder,
    Mount,
    Unmount,
    OpenInExplorer,
    Details,
    ReadOnlyWarning,
    Version,
    AboutDescription,
    Copyright,
    Close,
    PrerequisiteTitle,
    PrerequisiteSubtitle,
    ToContinue,
    PrerequisiteStepOne,
    PrerequisiteStepTwo,
    PrerequisiteStepThree,
    PrerequisiteNotice,
    DownloadWinFsp,
    Recheck,
    AutomaticLanguage,
}
impl TextKey {
    const ALL: [Self; 30] = [
        Self::AppTitle,
        Self::AppSubtitle,
        Self::ScanDrives,
        Self::OpenImage,
        Self::About,
        Self::Sources,
        Self::SourcesSubtitle,
        Self::SourcesEmpty,
        Self::FilesystemDetails,
        Self::OpenFilesystemImage,
        Self::ImagePlaceholder,
        Self::Mount,
        Self::Unmount,
        Self::OpenInExplorer,
        Self::Details,
        Self::ReadOnlyWarning,
        Self::Version,
        Self::AboutDescription,
        Self::Copyright,
        Self::Close,
        Self::PrerequisiteTitle,
        Self::PrerequisiteSubtitle,
        Self::ToContinue,
        Self::PrerequisiteStepOne,
        Self::PrerequisiteStepTwo,
        Self::PrerequisiteStepThree,
        Self::PrerequisiteNotice,
        Self::DownloadWinFsp,
        Self::Recheck,
        Self::AutomaticLanguage,
    ];
}

const EN: [&str; 30] = [
    "LinuxFS Manager",
    "Read Linux filesystems safely on Windows",
    "Scan Drives",
    "Open Image…",
    "About",
    "Sources",
    "Partitions and image files",
    "Scan your drives or open an image to begin.",
    "Filesystem details",
    "Open a filesystem image",
    "Image path (or use Open Image…)",
    "Mount",
    "Unmount",
    "Open in Explorer",
    "Details",
    "READ ONLY — source filesystems are never modified.",
    "Version",
    "LinuxFS Manager provides safe, read-only access to Ext2/3/4, SquashFS, and supported XFS images from Windows physical disks, partitions, and raw image files.",
    "LinuxFS Manager, @2026 Alfazen Inc. All rights reserved.",
    "Close",
    "WinFsp is required",
    "A Windows filesystem framework prerequisite",
    "To continue",
    "1. Download WinFsp from its official release page.",
    "2. Run the MSI installer and accept its driver installation.",
    "3. Return here and select Recheck.",
    "LinuxFS Manager does not download, install, or modify WinFsp for you.",
    "Download WinFsp",
    "Recheck",
    "Automatic (Windows)",
];
const FR: [&str; 30] = [
    "LinuxFS Manager",
    "Lisez les systèmes de fichiers Linux en toute sécurité sous Windows",
    "Analyser les disques",
    "Ouvrir une image…",
    "À propos",
    "Sources",
    "Partitions et fichiers image",
    "Analysez vos disques ou ouvrez une image pour commencer.",
    "Détails du système de fichiers",
    "Ouvrir une image de système de fichiers",
    "Chemin de l’image (ou utilisez Ouvrir une image…)",
    "Monter",
    "Démonter",
    "Ouvrir dans l’Explorateur",
    "Détails",
    "LECTURE SEULE — les systèmes de fichiers source ne sont jamais modifiés.",
    "Version",
    "LinuxFS Manager offre un accès sûr et en lecture seule aux images Ext2/3/4, SquashFS et XFS prises en charge depuis les disques Windows, partitions et fichiers image bruts.",
    "LinuxFS Manager, @2026 Alfazen Inc. Tous droits réservés.",
    "Fermer",
    "WinFsp est requis",
    "Prérequis du framework de système de fichiers Windows",
    "Pour continuer",
    "1. Téléchargez WinFsp depuis sa page officielle.",
    "2. Exécutez le programme MSI et acceptez l’installation du pilote.",
    "3. Revenez ici et sélectionnez Vérifier à nouveau.",
    "LinuxFS Manager ne télécharge, n’installe ni ne modifie WinFsp pour vous.",
    "Télécharger WinFsp",
    "Vérifier à nouveau",
    "Automatique (Windows)",
];
const DE: [&str; 30] = [
    "LinuxFS Manager",
    "Linux-Dateisysteme unter Windows sicher lesen",
    "Laufwerke scannen",
    "Image öffnen…",
    "Info",
    "Quellen",
    "Partitionen und Image-Dateien",
    "Scannen Sie Ihre Laufwerke oder öffnen Sie zum Start ein Image.",
    "Dateisystemdetails",
    "Dateisystem-Image öffnen",
    "Image-Pfad (oder Image öffnen… verwenden)",
    "Einbinden",
    "Aushängen",
    "Im Explorer öffnen",
    "Details",
    "NUR LESEN — Quelldateisysteme werden niemals verändert.",
    "Version",
    "LinuxFS Manager bietet sicheren schreibgeschützten Zugriff auf Ext2/3/4-, SquashFS- und unterstützte XFS-Images von Windows-Datenträgern, Partitionen und Raw-Image-Dateien.",
    "LinuxFS Manager, @2026 Alfazen Inc. Alle Rechte vorbehalten.",
    "Schließen",
    "WinFsp ist erforderlich",
    "Voraussetzung für das Windows-Dateisystemframework",
    "So geht es weiter",
    "1. Laden Sie WinFsp von der offiziellen Release-Seite herunter.",
    "2. Starten Sie das MSI-Installationsprogramm und akzeptieren Sie die Treiberinstallation.",
    "3. Kehren Sie hierher zurück und wählen Sie Erneut prüfen.",
    "LinuxFS Manager lädt WinFsp nicht für Sie herunter, installiert oder verändert es nicht.",
    "WinFsp herunterladen",
    "Erneut prüfen",
    "Automatisch (Windows)",
];
const ES: [&str; 30] = [
    "LinuxFS Manager",
    "Lea sistemas de archivos Linux de forma segura en Windows",
    "Analizar unidades",
    "Abrir imagen…",
    "Acerca de",
    "Fuentes",
    "Particiones y archivos de imagen",
    "Analice sus unidades o abra una imagen para comenzar.",
    "Detalles del sistema de archivos",
    "Abrir una imagen de sistema de archivos",
    "Ruta de imagen (o use Abrir imagen…)",
    "Montar",
    "Desmontar",
    "Abrir en el Explorador",
    "Detalles",
    "SOLO LECTURA — los sistemas de archivos de origen nunca se modifican.",
    "Versión",
    "LinuxFS Manager ofrece acceso seguro y de solo lectura a imágenes Ext2/3/4, SquashFS y XFS compatibles desde discos físicos, particiones y archivos de imagen sin procesar de Windows.",
    "LinuxFS Manager, @2026 Alfazen Inc. Todos los derechos reservados.",
    "Cerrar",
    "Se requiere WinFsp",
    "Requisito previo del marco de sistemas de archivos de Windows",
    "Para continuar",
    "1. Descargue WinFsp desde su página oficial.",
    "2. Ejecute el instalador MSI y acepte la instalación del controlador.",
    "3. Regrese aquí y seleccione Comprobar de nuevo.",
    "LinuxFS Manager no descarga, instala ni modifica WinFsp por usted.",
    "Descargar WinFsp",
    "Comprobar de nuevo",
    "Automático (Windows)",
];
const PT_BR: [&str; 30] = [
    "LinuxFS Manager",
    "Leia sistemas de arquivos Linux com segurança no Windows",
    "Verificar unidades",
    "Abrir imagem…",
    "Sobre",
    "Fontes",
    "Partições e arquivos de imagem",
    "Verifique suas unidades ou abra uma imagem para começar.",
    "Detalhes do sistema de arquivos",
    "Abrir uma imagem de sistema de arquivos",
    "Caminho da imagem (ou use Abrir imagem…)",
    "Montar",
    "Desmontar",
    "Abrir no Explorador",
    "Detalhes",
    "SOMENTE LEITURA — os sistemas de arquivos de origem nunca são modificados.",
    "Versão",
    "LinuxFS Manager fornece acesso seguro e somente leitura a imagens Ext2/3/4, SquashFS e XFS compatíveis de discos físicos, partições e arquivos de imagem bruta do Windows.",
    "LinuxFS Manager, @2026 Alfazen Inc. Todos os direitos reservados.",
    "Fechar",
    "WinFsp é necessário",
    "Pré-requisito da estrutura de sistema de arquivos do Windows",
    "Para continuar",
    "1. Baixe o WinFsp na página oficial de lançamentos.",
    "2. Execute o instalador MSI e aceite a instalação do driver.",
    "3. Volte aqui e selecione Verificar novamente.",
    "LinuxFS Manager não baixa, instala nem modifica o WinFsp para você.",
    "Baixar WinFsp",
    "Verificar novamente",
    "Automático (Windows)",
];
const IT: [&str; 30] = [
    "LinuxFS Manager",
    "Leggi i filesystem Linux in modo sicuro su Windows",
    "Analizza unità",
    "Apri immagine…",
    "Informazioni",
    "Origini",
    "Partizioni e file immagine",
    "Analizza le unità o apri un’immagine per iniziare.",
    "Dettagli del filesystem",
    "Apri un’immagine del filesystem",
    "Percorso immagine (o usa Apri immagine…)",
    "Monta",
    "Smonta",
    "Apri in Esplora file",
    "Dettagli",
    "SOLA LETTURA — i filesystem di origine non vengono mai modificati.",
    "Versione",
    "LinuxFS Manager fornisce accesso sicuro in sola lettura a immagini Ext2/3/4, SquashFS e XFS supportate da dischi fisici, partizioni e file immagine raw di Windows.",
    "LinuxFS Manager, @2026 Alfazen Inc. Tutti i diritti riservati.",
    "Chiudi",
    "WinFsp è richiesto",
    "Prerequisito del framework di filesystem Windows",
    "Per continuare",
    "1. Scarica WinFsp dalla pagina ufficiale delle versioni.",
    "2. Esegui l’installatore MSI e accetta l’installazione del driver.",
    "3. Torna qui e seleziona Ricontrolla.",
    "LinuxFS Manager non scarica, installa né modifica WinFsp per te.",
    "Scarica WinFsp",
    "Ricontrolla",
    "Automatico (Windows)",
];
const PL: [&str; 30] = [
    "LinuxFS Manager",
    "Bezpiecznie odczytuj systemy plików Linux w Windows",
    "Skanuj dyski",
    "Otwórz obraz…",
    "Informacje",
    "Źródła",
    "Partycje i pliki obrazów",
    "Zeskanuj dyski lub otwórz obraz, aby rozpocząć.",
    "Szczegóły systemu plików",
    "Otwórz obraz systemu plików",
    "Ścieżka obrazu (lub użyj Otwórz obraz…)",
    "Zamontuj",
    "Odmontuj",
    "Otwórz w Eksploratorze",
    "Szczegóły",
    "TYLKO DO ODCZYTU — źródłowe systemy plików nigdy nie są modyfikowane.",
    "Wersja",
    "LinuxFS Manager zapewnia bezpieczny dostęp tylko do odczytu do obsługiwanych obrazów Ext2/3/4, SquashFS i XFS z dysków fizycznych Windows, partycji i surowych plików obrazów.",
    "LinuxFS Manager, @2026 Alfazen Inc. Wszelkie prawa zastrzeżone.",
    "Zamknij",
    "WinFsp jest wymagany",
    "Wymaganie wstępne frameworka systemu plików Windows",
    "Aby kontynuować",
    "1. Pobierz WinFsp z oficjalnej strony wydań.",
    "2. Uruchom instalator MSI i zaakceptuj instalację sterownika.",
    "3. Wróć tutaj i wybierz Sprawdź ponownie.",
    "LinuxFS Manager nie pobiera, nie instaluje ani nie modyfikuje WinFsp za Ciebie.",
    "Pobierz WinFsp",
    "Sprawdź ponownie",
    "Automatycznie (Windows)",
];
const RU: [&str; 30] = [
    "LinuxFS Manager",
    "Безопасный доступ к файловым системам Linux в Windows",
    "Сканировать диски",
    "Открыть образ…",
    "О программе",
    "Источники",
    "Разделы и файлы образов",
    "Просканируйте диски или откройте образ, чтобы начать.",
    "Сведения о файловой системе",
    "Открыть образ файловой системы",
    "Путь к образу (или используйте Открыть образ…)",
    "Подключить",
    "Отключить",
    "Открыть в Проводнике",
    "Сведения",
    "ТОЛЬКО ЧТЕНИЕ — исходные файловые системы никогда не изменяются.",
    "Версия",
    "LinuxFS Manager предоставляет безопасный доступ только для чтения к поддерживаемым образам Ext2/3/4, SquashFS и XFS с физических дисков Windows, разделов и необработанных файлов образов.",
    "LinuxFS Manager, @2026 Alfazen Inc. Все права защищены.",
    "Закрыть",
    "Требуется WinFsp",
    "Необходимый компонент инфраструктуры файловых систем Windows",
    "Чтобы продолжить",
    "1. Загрузите WinFsp с официальной страницы выпуска.",
    "2. Запустите установщик MSI и подтвердите установку драйвера.",
    "3. Вернитесь сюда и выберите Повторить проверку.",
    "LinuxFS Manager не загружает, не устанавливает и не изменяет WinFsp за вас.",
    "Загрузить WinFsp",
    "Повторить проверку",
    "Автоматически (Windows)",
];
const ZH_CN: [&str; 30] = [
    "LinuxFS Manager",
    "在 Windows 上安全读取 Linux 文件系统",
    "扫描驱动器",
    "打开映像…",
    "关于",
    "源",
    "分区和映像文件",
    "扫描驱动器或打开映像以开始。",
    "文件系统详细信息",
    "打开文件系统映像",
    "映像路径（或使用“打开映像…”）",
    "挂载",
    "卸载",
    "在资源管理器中打开",
    "详细信息",
    "只读 — 永不修改源文件系统。",
    "版本",
    "LinuxFS Manager 可从 Windows 物理磁盘、分区和原始映像文件安全地只读访问 Ext2/3/4、SquashFS 和受支持的 XFS 映像。",
    "LinuxFS Manager, @2026 Alfazen Inc. 保留所有权利。",
    "关闭",
    "需要 WinFsp",
    "Windows 文件系统框架先决条件",
    "继续操作",
    "1. 从 WinFsp 官方发布页面下载。",
    "2. 运行 MSI 安装程序并接受驱动程序安装。",
    "3. 返回此处并选择“重新检查”。",
    "LinuxFS Manager 不会为您下载、安装或修改 WinFsp。",
    "下载 WinFsp",
    "重新检查",
    "自动（Windows）",
];
const ZH_TW: [&str; 30] = [
    "LinuxFS Manager",
    "在 Windows 上安全讀取 Linux 檔案系統",
    "掃描磁碟機",
    "開啟映像檔…",
    "關於",
    "來源",
    "分割區和映像檔",
    "掃描磁碟機或開啟映像檔以開始。",
    "檔案系統詳細資料",
    "開啟檔案系統映像檔",
    "映像檔路徑（或使用「開啟映像檔…」）",
    "掛載",
    "卸載",
    "在檔案總管中開啟",
    "詳細資料",
    "唯讀 — 永不修改來源檔案系統。",
    "版本",
    "LinuxFS Manager 可從 Windows 實體磁碟、分割區和原始映像檔安全地唯讀存取 Ext2/3/4、SquashFS 和支援的 XFS 映像檔。",
    "LinuxFS Manager, @2026 Alfazen Inc. 保留所有權利。",
    "關閉",
    "需要 WinFsp",
    "Windows 檔案系統框架必要條件",
    "繼續",
    "1. 從 WinFsp 官方版本頁面下載。",
    "2. 執行 MSI 安裝程式並接受驅動程式安裝。",
    "3. 返回此處並選取「重新檢查」。",
    "LinuxFS Manager 不會為您下載、安裝或修改 WinFsp。",
    "下載 WinFsp",
    "重新檢查",
    "自動（Windows）",
];
const JA: [&str; 30] = [
    "LinuxFS Manager",
    "Windows で Linux ファイルシステムを安全に読み取る",
    "ドライブをスキャン",
    "イメージを開く…",
    "バージョン情報",
    "ソース",
    "パーティションとイメージファイル",
    "ドライブをスキャンするか、イメージを開いて開始します。",
    "ファイルシステムの詳細",
    "ファイルシステムイメージを開く",
    "イメージのパス（または「イメージを開く…」を使用）",
    "マウント",
    "アンマウント",
    "エクスプローラーで開く",
    "詳細",
    "読み取り専用 — ソースファイルシステムは変更されません。",
    "バージョン",
    "LinuxFS Manager は、Windows の物理ディスク、パーティション、raw イメージファイルから Ext2/3/4、SquashFS、対応 XFS イメージへの安全な読み取り専用アクセスを提供します。",
    "LinuxFS Manager, @2026 Alfazen Inc. All rights reserved.",
    "閉じる",
    "WinFsp が必要です",
    "Windows ファイルシステムフレームワークの前提条件",
    "続行するには",
    "1. 公式リリースページから WinFsp をダウンロードします。",
    "2. MSI インストーラーを実行し、ドライバーのインストールを許可します。",
    "3. ここに戻り、「再確認」を選択します。",
    "LinuxFS Manager は WinFsp をダウンロード、インストール、変更しません。",
    "WinFsp をダウンロード",
    "再確認",
    "自動（Windows）",
];
const KO: [&str; 30] = [
    "LinuxFS Manager",
    "Windows에서 Linux 파일 시스템을 안전하게 읽기",
    "드라이브 검색",
    "이미지 열기…",
    "정보",
    "원본",
    "파티션 및 이미지 파일",
    "드라이브를 검색하거나 이미지를 열어 시작하세요.",
    "파일 시스템 세부 정보",
    "파일 시스템 이미지 열기",
    "이미지 경로(또는 이미지 열기… 사용)",
    "마운트",
    "마운트 해제",
    "탐색기에서 열기",
    "세부 정보",
    "읽기 전용 — 원본 파일 시스템은 절대 수정되지 않습니다.",
    "버전",
    "LinuxFS Manager는 Windows 물리 디스크, 파티션 및 원시 이미지 파일에서 Ext2/3/4, SquashFS 및 지원되는 XFS 이미지에 대한 안전한 읽기 전용 액세스를 제공합니다.",
    "LinuxFS Manager, @2026 Alfazen Inc. All rights reserved.",
    "닫기",
    "WinFsp가 필요합니다",
    "Windows 파일 시스템 프레임워크 필수 구성 요소",
    "계속하려면",
    "1. 공식 릴리스 페이지에서 WinFsp를 다운로드하세요.",
    "2. MSI 설치 관리자를 실행하고 드라이버 설치를 허용하세요.",
    "3. 여기로 돌아와 다시 확인을 선택하세요.",
    "LinuxFS Manager는 WinFsp를 다운로드, 설치 또는 수정하지 않습니다.",
    "WinFsp 다운로드",
    "다시 확인",
    "자동(Windows)",
];

fn text(language: UiLanguage, key: TextKey) -> &'static str {
    let table = match language {
        UiLanguage::English => &EN,
        UiLanguage::French => &FR,
        UiLanguage::German => &DE,
        UiLanguage::Spanish => &ES,
        UiLanguage::PortugueseBrazil => &PT_BR,
        UiLanguage::Italian => &IT,
        UiLanguage::Polish => &PL,
        UiLanguage::Russian => &RU,
        UiLanguage::ChineseSimplified => &ZH_CN,
        UiLanguage::ChineseTraditional => &ZH_TW,
        UiLanguage::Japanese => &JA,
        UiLanguage::Korean => &KO,
    };
    table[key as usize]
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum DynamicKey {
    Ready,
    RefreshFailed,
    Mounting,
    Mounted,
    Unmounting,
    Unmounted,
    MountFailed,
    UnmountFailed,
    ExplorerOpened,
    ExistingMountAvailable,
}

fn dynamic(language: UiLanguage, key: DynamicKey, value: &str) -> String {
    let table = match language {
        UiLanguage::English => &DYNAMIC_EN,
        UiLanguage::French => &DYNAMIC_FR,
        UiLanguage::German => &DYNAMIC_DE,
        UiLanguage::Spanish => &DYNAMIC_ES,
        UiLanguage::PortugueseBrazil => &DYNAMIC_PT_BR,
        UiLanguage::Italian => &DYNAMIC_IT,
        UiLanguage::Polish => &DYNAMIC_PL,
        UiLanguage::Russian => &DYNAMIC_RU,
        UiLanguage::ChineseSimplified => &DYNAMIC_ZH_CN,
        UiLanguage::ChineseTraditional => &DYNAMIC_ZH_TW,
        UiLanguage::Japanese => &DYNAMIC_JA,
        UiLanguage::Korean => &DYNAMIC_KO,
    };
    table[key as usize].replace("{value}", value)
}

const DYNAMIC_EN: [&str; 10] = [
    "Ready.",
    "Refresh failed: {value}",
    "Mounting read-only…",
    "Mounted read-only on {value} — source unchanged",
    "Unmounting…",
    "Unmount completed",
    "Mount failed: {value}",
    "Unmount failed: {value}",
    "Opened {value} in Explorer",
    "Existing mount remains available: {value}",
];
const DYNAMIC_FR: [&str; 10] = [
    "Prêt.",
    "Actualisation échouée : {value}",
    "Montage en lecture seule…",
    "Monté en lecture seule sur {value} — source inchangée",
    "Démontage…",
    "Démontage terminé",
    "Échec du montage : {value}",
    "Échec du démontage : {value}",
    "{value} ouvert dans l’Explorateur",
    "Le montage existant reste disponible : {value}",
];
const DYNAMIC_DE: [&str; 10] = [
    "Bereit.",
    "Aktualisierung fehlgeschlagen: {value}",
    "Schreibgeschützt einbinden…",
    "Schreibgeschützt auf {value} eingebunden — Quelle unverändert",
    "Aushängen…",
    "Aushängen abgeschlossen",
    "Einbinden fehlgeschlagen: {value}",
    "Aushängen fehlgeschlagen: {value}",
    "{value} im Explorer geöffnet",
    "Vorhandene Einbindung bleibt verfügbar: {value}",
];
const DYNAMIC_ES: [&str; 10] = [
    "Listo.",
    "Error de actualización: {value}",
    "Montando en solo lectura…",
    "Montado en solo lectura en {value} — origen sin cambios",
    "Desmontando…",
    "Desmontaje completado",
    "Error al montar: {value}",
    "Error al desmontar: {value}",
    "{value} abierto en el Explorador",
    "El montaje existente sigue disponible: {value}",
];
const DYNAMIC_PT_BR: [&str; 10] = [
    "Pronto.",
    "Falha ao atualizar: {value}",
    "Montando somente leitura…",
    "Montado somente leitura em {value} — origem inalterada",
    "Desmontando…",
    "Desmontagem concluída",
    "Falha ao montar: {value}",
    "Falha ao desmontar: {value}",
    "{value} aberto no Explorador",
    "A montagem existente permanece disponível: {value}",
];
const DYNAMIC_IT: [&str; 10] = [
    "Pronto.",
    "Aggiornamento non riuscito: {value}",
    "Montaggio in sola lettura…",
    "Montato in sola lettura su {value} — origine invariata",
    "Smontaggio…",
    "Smontaggio completato",
    "Montaggio non riuscito: {value}",
    "Smontaggio non riuscito: {value}",
    "{value} aperto in Esplora file",
    "Il montaggio esistente rimane disponibile: {value}",
];
const DYNAMIC_PL: [&str; 10] = [
    "Gotowe.",
    "Odświeżanie nie powiodło się: {value}",
    "Montowanie tylko do odczytu…",
    "Zamontowano tylko do odczytu w {value} — źródło niezmienione",
    "Odmontowywanie…",
    "Odmontowanie zakończone",
    "Montowanie nie powiodło się: {value}",
    "Odmontowanie nie powiodło się: {value}",
    "Otwarto {value} w Eksploratorze",
    "Istniejące montowanie pozostaje dostępne: {value}",
];
const DYNAMIC_RU: [&str; 10] = [
    "Готово.",
    "Не удалось обновить: {value}",
    "Подключение только для чтения…",
    "Подключено только для чтения в {value} — источник не изменён",
    "Отключение…",
    "Отключение завершено",
    "Не удалось подключить: {value}",
    "Не удалось отключить: {value}",
    "{value} открыт в Проводнике",
    "Существующее подключение остаётся доступным: {value}",
];
const DYNAMIC_ZH_CN: [&str; 10] = [
    "就绪。",
    "刷新失败：{value}",
    "正在以只读方式挂载…",
    "已在 {value} 以只读方式挂载 — 源未更改",
    "正在卸载…",
    "卸载完成",
    "挂载失败：{value}",
    "卸载失败：{value}",
    "已在资源管理器中打开 {value}",
    "现有挂载仍可用：{value}",
];
const DYNAMIC_ZH_TW: [&str; 10] = [
    "就緒。",
    "重新整理失敗：{value}",
    "正在以唯讀方式掛載…",
    "已在 {value} 以唯讀方式掛載 — 來源未變更",
    "正在卸載…",
    "卸載完成",
    "掛載失敗：{value}",
    "卸載失敗：{value}",
    "已在檔案總管中開啟 {value}",
    "現有掛載仍可用：{value}",
];
const DYNAMIC_JA: [&str; 10] = [
    "準備完了。",
    "更新に失敗しました: {value}",
    "読み取り専用でマウント中…",
    "{value} に読み取り専用でマウント済み — ソースは変更されていません",
    "アンマウント中…",
    "アンマウントが完了しました",
    "マウントに失敗しました: {value}",
    "アンマウントに失敗しました: {value}",
    "エクスプローラーで {value} を開きました",
    "既存のマウントは引き続き利用できます: {value}",
];
const DYNAMIC_KO: [&str; 10] = [
    "준비되었습니다.",
    "새로 고침 실패: {value}",
    "읽기 전용으로 마운트하는 중…",
    "{value}에 읽기 전용으로 마운트됨 — 원본이 변경되지 않았습니다",
    "마운트 해제 중…",
    "마운트 해제 완료",
    "마운트 실패: {value}",
    "마운트 해제 실패: {value}",
    "탐색기에서 {value} 열림",
    "기존 마운트는 계속 사용할 수 있습니다: {value}",
];

#[cfg(test)]
mod tests {
    use super::{UiLanguage, catalog, language_from_self_name, load_catalog, resolve_language};

    #[test]
    fn resolver_prefers_a_supported_saved_override() {
        assert_eq!(resolve_language(Some("ko-KR"), "fr-FR"), UiLanguage::Korean);
    }

    #[test]
    fn resolver_matches_windows_base_language_then_falls_back_to_english() {
        assert_eq!(resolve_language(None, "de-AT"), UiLanguage::German);
        assert_eq!(resolve_language(None, "nl-NL"), UiLanguage::English);
    }

    #[test]
    fn every_catalog_contains_all_message_keys() {
        for language in UiLanguage::ALL {
            assert!(catalog(language).is_complete());
        }
    }

    #[test]
    fn dynamic_status_keeps_the_mount_point_but_uses_the_selected_language() {
        let status = catalog(UiLanguage::Japanese).mounted("Z:");
        assert!(status.contains("Z:"));
        assert_ne!(status, "Mounted read-only on Z: — source unchanged");
    }

    #[test]
    fn selector_self_name_resolves_to_its_language() {
        assert_eq!(
            language_from_self_name("繁體中文"),
            Some(UiLanguage::ChineseTraditional)
        );
    }

    #[test]
    fn empty_source_copy_is_localized_for_spanish_and_chinese() {
        assert_eq!(
            catalog(UiLanguage::Spanish).no_source_loaded(),
            "No hay fuente cargada"
        );
        assert_eq!(
            catalog(UiLanguage::ChineseSimplified).no_source_loaded(),
            "未加载源"
        );
        assert_eq!(
            catalog(UiLanguage::ChineseTraditional).no_source_loaded(),
            "未載入來源"
        );
    }

    #[test]
    fn external_file_overrides_copy_only_for_its_declared_language() {
        let directory = temp_directory();
        std::fs::create_dir_all(&directory).expect("create locale directory");
        std::fs::write(
            directory.join("es-ES.toml"),
            "language = \"es-ES\"\n[strings]\napp_title = \"Gestor LinuxFS\"\n",
        )
        .expect("write locale");

        let spanish = load_catalog(UiLanguage::Spanish, &directory);
        let english = load_catalog(UiLanguage::English, &directory);
        assert_eq!(
            spanish.text("app_title", spanish.copy.app_title),
            "Gestor LinuxFS"
        );
        assert_eq!(
            english.text("app_title", english.copy.app_title),
            "LinuxFS Manager"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn east_asian_languages_request_the_matching_windows_ui_font() {
        assert_eq!(
            UiLanguage::ChineseSimplified.default_font_family(),
            "Microsoft YaHei UI"
        );
        assert_eq!(
            UiLanguage::ChineseTraditional.default_font_family(),
            "Microsoft JhengHei UI"
        );
        assert_eq!(UiLanguage::Japanese.default_font_family(), "Yu Gothic UI");
        assert_eq!(UiLanguage::Korean.default_font_family(), "Malgun Gothic");
    }

    fn temp_directory() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("linuxfs-manager-locales-{nonce}"))
    }
}
