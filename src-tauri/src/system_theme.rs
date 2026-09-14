//! 桌面外观（深色 / 浅色）与 WebView 的同步层。
//!
//! WebKitGTK 只根据 GTK 的 `gtk-theme-name` / `gtk-application-prefer-dark-theme`
//! 判断 `prefers-color-scheme`，而 GNOME 在深色模式下仍把 `gtk-theme-name` 保持为
//! `Adwaita`，于是「跟随系统」的界面主题会误判成浅色。这里读取 XDG Desktop Portal
//! 的外观设置（`org.freedesktop.appearance` / `color-scheme`）并同步给 GTK 的深色
//! 设置，让面板配色与桌面一致；桌面切换深色 / 浅色时门户会发送 `SettingChanged`
//! 信号，WebView 会随之收到 `prefers-color-scheme` 变更事件。

/// 非 Linux 平台的 WebView 已经能正确报告系统外观，无需额外同步。
#[cfg(not(target_os = "linux"))]
pub(crate) fn start(_app: &tauri::AppHandle) {}

#[cfg(target_os = "linux")]
pub(crate) use linux::start;

#[cfg(target_os = "linux")]
mod linux {
    use std::thread;

    use gtk::prelude::GtkSettingsExt;
    use tauri::AppHandle;
    use zbus::{
        blocking::{Connection, Proxy},
        zvariant::{OwnedValue, Value},
    };

    /// 门户外观设置的命名空间与键名。
    const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";
    const COLOR_SCHEME_KEY: &str = "color-scheme";

    /// 门户 Settings 接口的服务名、对象路径与接口名。
    const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
    const PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS_INTERFACE: &str = "org.freedesktop.portal.Settings";

    /// 桌面外观偏好，只保留门户明确表态的取值。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Appearance {
        Dark,
        Light,
    }

    impl Appearance {
        /// 门户 `color-scheme` 的取值：0 表示未表态，1 表示深色，2 表示浅色。
        fn from_portal_code(code: u32) -> Option<Self> {
            match code {
                1 => Some(Self::Dark),
                2 => Some(Self::Light),
                _ => None,
            }
        }

        /// 读取外观取值：门户的 method 返回值与变更信号都会把取值包在变体里。
        fn from_value(value: &Value<'_>) -> Option<Self> {
            match value {
                Value::U32(code) => Self::from_portal_code(*code),
                Value::Value(inner) => Self::from_value(inner),
                _ => None,
            }
        }

        fn prefers_dark(self) -> bool {
            matches!(self, Self::Dark)
        }
    }

    /// 读取桌面外观并让面板跟随，随后在后台线程监听桌面切换。
    ///
    /// 启动阶段的一次同步在主线程执行，保证 WebView 首次加载页面时
    /// `prefers-color-scheme` 就是正确取值。
    pub(crate) fn start(app: &AppHandle) {
        let connection = match Connection::session() {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("[theme] 会话总线不可用，界面主题继续由 WebView 自行判断：{error}");
                return;
            }
        };

        match read_appearance(&connection) {
            // 主线程直接同步，WebView 首次加载页面时 prefers-color-scheme 就是正确取值。
            Ok(Some(appearance)) => set_gtk_prefers_dark(appearance.prefers_dark()),
            Ok(None) => {}
            Err(error) => eprintln!("[theme] 桌面外观门户不可用，界面主题由 WebView 判断：{error}"),
        }

        let watcher_app = app.clone();
        let watcher = thread::Builder::new()
            .name("system-theme-watcher".into())
            .spawn(move || {
                if let Err(error) = watch_appearance(&watcher_app, &connection) {
                    eprintln!("[theme] 桌面外观监听已停止：{error}");
                }
            });

        if let Err(error) = watcher {
            eprintln!("[theme] 无法启动桌面外观监听线程：{error}");
        }
    }

    /// 读取 `org.freedesktop.appearance` 下的 `color-scheme`。
    fn read_appearance(connection: &Connection) -> zbus::Result<Option<Appearance>> {
        let proxy = Proxy::new(
            connection,
            PORTAL_BUS_NAME,
            PORTAL_OBJECT_PATH,
            SETTINGS_INTERFACE,
        )?;
        let value: OwnedValue = proxy.call("Read", &(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY))?;
        Ok(Appearance::from_value(&value))
    }

    /// 监听门户的外观设置变更，桌面切换深色 / 浅色时同步给 GTK。
    fn watch_appearance(app: &AppHandle, connection: &Connection) -> zbus::Result<()> {
        let proxy = Proxy::new(
            connection,
            PORTAL_BUS_NAME,
            PORTAL_OBJECT_PATH,
            SETTINGS_INTERFACE,
        )?;

        for message in proxy.receive_signal("SettingChanged")? {
            let Ok((namespace, key, value)) =
                message.body().deserialize::<(String, String, OwnedValue)>()
            else {
                continue;
            };
            if namespace != APPEARANCE_NAMESPACE || key != COLOR_SCHEME_KEY {
                continue;
            }

            if let Some(appearance) = Appearance::from_value(&value) {
                apply_gtk_appearance_on_main_thread(app, appearance);
            }
        }

        Ok(())
    }

    /// 监听线程不在 GTK 主线程上，同步 GTK 设置需要切回主线程执行。
    fn apply_gtk_appearance_on_main_thread(app: &AppHandle, appearance: Appearance) {
        let applied = app.run_on_main_thread(move || set_gtk_prefers_dark(appearance.prefers_dark()));
        if let Err(error) = applied {
            eprintln!("[theme] 同步桌面外观到 GTK 失败：{error}");
        }
    }

    /// `gtk-application-prefer-dark-theme` 是 WebKitGTK 判断深色外观的依据；
    /// GTK 只能在主线程访问，调用方需保证当前位于 GTK 主线程。
    fn set_gtk_prefers_dark(prefers_dark: bool) {
        let Some(settings) = gtk::Settings::default() else {
            eprintln!("[theme] 无法访问 GTK 设置，桌面外观未同步");
            return;
        };
        if settings.is_gtk_application_prefer_dark_theme() == prefers_dark {
            return;
        }

        settings.set_gtk_application_prefer_dark_theme(prefers_dark);
        eprintln!(
            "[theme] 已同步桌面外观：{}",
            if prefers_dark { "深色" } else { "浅色" }
        );
    }

    #[cfg(test)]
    mod tests {
        use super::{
            Appearance, APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY, PORTAL_OBJECT_PATH,
            SETTINGS_INTERFACE,
        };
        use zbus::zvariant::{OwnedValue, Value};

        #[test]
        fn maps_portal_color_scheme_codes() {
            assert_eq!(Appearance::from_portal_code(1), Some(Appearance::Dark));
            assert_eq!(Appearance::from_portal_code(2), Some(Appearance::Light));
            assert_eq!(Appearance::from_portal_code(0), None);
            assert_eq!(Appearance::from_portal_code(3), None);
        }

        // 门户把外观取值放在变体里，变更信号按 `(ssv)` 解析。
        #[test]
        fn reads_appearance_from_setting_changed_signal() {
            let message =
                zbus::Message::signal(PORTAL_OBJECT_PATH, SETTINGS_INTERFACE, "SettingChanged")
                    .expect("signal builder")
                    .build(&(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY, Value::from(1u32)))
                    .expect("signal message");

            let (namespace, key, value) = message
                .body()
                .deserialize::<(String, String, OwnedValue)>()
                .expect("signal body");

            assert_eq!(namespace, APPEARANCE_NAMESPACE);
            assert_eq!(key, COLOR_SCHEME_KEY);
            assert_eq!(Appearance::from_value(&value), Some(Appearance::Dark));
        }
    }
}
