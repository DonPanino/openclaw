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
    WebChat,
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
        "webchat" | "chat" => Some(DeepLink::WebChat),
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
            Some(DeepLink::WebChat)
        ));
    }
}
