use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLink {
    Agent {
        message: String,
        session: Option<String>,
        thinking: Option<String>,
        deliver: Option<bool>,
    },
    Dashboard,
    WebChat {
        session: Option<String>,
    },
    Settings,
    Canvas {
        session: Option<String>,
    },
    Gateway {
        host: Option<String>,
        port: Option<u16>,
    },
}

pub fn parse_deep_link(raw: &str) -> Option<DeepLink> {
    let url = Url::parse(raw).ok()?;
    if url.scheme() != "openclaw" {
        return None;
    }
    match url.host_str()? {
        "agent" => {
            let message = url
                .query_pairs()
                .find(|(k, _)| k == "message")
                .map(|(_, v)| v.into_owned())?;
            let session = url
                .query_pairs()
                .find(|(k, _)| k == "session")
                .map(|(_, v)| v.into_owned());
            let thinking = url
                .query_pairs()
                .find(|(k, _)| k == "thinking")
                .map(|(_, v)| v.into_owned());
            let deliver = url
                .query_pairs()
                .find(|(k, _)| k == "deliver")
                .and_then(|(_, v)| v.parse().ok());
            Some(DeepLink::Agent {
                message,
                session,
                thinking,
                deliver,
            })
        }
        "dashboard" => Some(DeepLink::Dashboard),
        "webchat" | "chat" => {
            let session = url
                .query_pairs()
                .find(|(k, _)| k == "session" || k == "sessionKey")
                .map(|(_, v)| v.into_owned());
            Some(DeepLink::WebChat { session })
        }
        "settings" => Some(DeepLink::Settings),
        "canvas" => {
            let session = url
                .query_pairs()
                .find(|(k, _)| k == "session" || k == "sessionKey")
                .map(|(_, v)| v.into_owned());
            Some(DeepLink::Canvas { session })
        }
        "gateway" => {
            let host = url
                .query_pairs()
                .find(|(k, _)| k == "host")
                .map(|(_, v)| v.into_owned());
            let port = url
                .query_pairs()
                .find(|(k, _)| k == "port")
                .and_then(|(_, v)| v.parse().ok());
            Some(DeepLink::Gateway { host, port })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_link() {
        let link = parse_deep_link("openclaw://agent?message=hello").unwrap();
        assert!(matches!(link, DeepLink::Agent { message, .. } if message == "hello"));
    }

    #[test]
    fn parses_dashboard() {
        assert!(matches!(
            parse_deep_link("openclaw://dashboard"),
            Some(DeepLink::Dashboard)
        ));
    }

    #[test]
    fn parses_webchat() {
        assert!(matches!(
            parse_deep_link("openclaw://webchat"),
            Some(DeepLink::WebChat { session: None })
        ));
        let link = parse_deep_link("openclaw://webchat?session=agent:main:main").unwrap();
        assert!(matches!(
            link,
            DeepLink::WebChat {
                session: Some(s),
            } if s == "agent:main:main"
        ));
    }

    #[test]
    fn parses_settings() {
        assert!(matches!(
            parse_deep_link("openclaw://settings"),
            Some(DeepLink::Settings)
        ));
    }

    #[test]
    fn parses_canvas_session() {
        let link = parse_deep_link("openclaw://canvas?session=desk").unwrap();
        assert!(matches!(
            link,
            DeepLink::Canvas {
                session: Some(s),
            } if s == "desk"
        ));
    }

    #[test]
    fn parses_agent_session() {
        let link = parse_deep_link("openclaw://agent?message=hi&session=main").unwrap();
        assert!(matches!(
            link,
            DeepLink::Agent {
                message,
                session: Some(s),
                ..
            } if message == "hi" && s == "main"
        ));
    }

    #[test]
    fn parses_gateway_host_port() {
        let link = parse_deep_link("openclaw://gateway?host=192.168.1.5&port=18789").unwrap();
        assert!(matches!(
            link,
            DeepLink::Gateway {
                host: Some(h),
                port: Some(18789),
            } if h == "192.168.1.5"
        ));
    }
}
