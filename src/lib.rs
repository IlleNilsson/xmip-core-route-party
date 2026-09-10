#![forbid(unsafe_code)]

//! The party route technology — a technology of `xmip-core-route`.
//!
//! A Subscription's filter names properties, and each property is read from
//! one source. This source reads who sent the Message and who will receive it:
//! `party:sender` and `party:receiver`, each the Party's identifier as text,
//! or nothing promoted where no Party was resolved — an unrecognised caller is
//! still authenticated or refused on its own terms, and a Party is a shortcut
//! to an identity, not a permission (ADR-0019 clause 4). ADR-0046.
//!
//! **Where the two are read from.** The runtime's arrival promotes what the
//! gates concluded under `xmip.`-prefixed context keys, so a Contract
//! promoting `Party` cannot collide with Xmip promoting one. `party:sender`
//! reads `xmip.party`, the accountable Party of the transmission — the one
//! `promote_identity` in `xmip-core-runtime` writes from the transport
//! identity. `party:receiver` reads `xmip.party.receiver`, which nothing
//! writes yet: the receiving Party is resolved on the way out, by Send
//! preparation, and that key is the name it is to be written under. This
//! paragraph is the record of that convention.
//!
//! A route technology does not decide anything: it reads.

use context::ContextValue;
use message::Message;
use route::{Source, SourceError};

/// The manifest leaf and the prefix a property carries.
pub const TECHNOLOGY: &str = "party";

/// The context key the accountable Party of the transmission is promoted
/// under, by the runtime's arrival.
pub const SENDER_KEY: &str = "xmip.party";

/// The context key the receiving Party is to be promoted under, by Send
/// preparation.
pub const RECEIVER_KEY: &str = "xmip.party.receiver";

/// The two names this technology reads.
pub const NAMES: [&str; 2] = ["sender", "receiver"];

/// Reads `party:sender` and `party:receiver`.
pub struct PartySource;

impl Source for PartySource {
    fn technology(&self) -> &'static str {
        TECHNOLOGY
    }

    fn read(&self, message: &Message, name: &str) -> Result<Option<String>, SourceError> {
        let key = match name {
            "sender" => SENDER_KEY,
            "receiver" => RECEIVER_KEY,
            other => {
                return Err(SourceError::new(
                    TECHNOLOGY,
                    other,
                    format!(
                        "not a party a Message has; the parties are {}",
                        NAMES.join(" and ")
                    ),
                ));
            }
        };

        match message.context().get(key) {
            None | Some(ContextValue::Null) => Ok(None),
            Some(ContextValue::Binary(bytes)) => Err(SourceError::new(
                TECHNOLOGY,
                name,
                format!(
                    "{key} holds {} bytes, and bytes are not routable as text",
                    bytes.len()
                ),
            )),
            Some(value) => Ok(route::text_of(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::MessageContext;
    use message::MessageTreatment;
    use route::{Predicate, Value};
    use xcore::{MessageId, PartyId};

    fn message(context: MessageContext) -> Message {
        Message::received(
            MessageId::new(1),
            Vec::new(),
            context,
            MessageTreatment::default(),
        )
    }

    fn resolved() -> Message {
        // The runtime writes the Party's identifier in its canonical form.
        message(
            MessageContext::new()
                .with_value(SENDER_KEY, ContextValue::Text(PartyId::new(42).to_string()))
                .with_value(RECEIVER_KEY, ContextValue::Text("partner-x".into())),
        )
    }

    #[test]
    fn the_sender_is_the_accountable_party_the_runtime_promoted() {
        assert_eq!(
            PartySource.read(&resolved(), "sender").expect("readable"),
            Some(PartyId::new(42).to_string())
        );
        assert_eq!(
            PartySource.read(&resolved(), "receiver").expect("readable"),
            Some("partner-x".into())
        );
    }

    #[test]
    fn no_party_resolved_is_nothing_promoted_not_an_error() {
        let anonymous = message(MessageContext::new());
        assert_eq!(
            PartySource.read(&anonymous, "sender").expect("readable"),
            None
        );
        assert_eq!(
            PartySource.read(&anonymous, "receiver").expect("readable"),
            None
        );
    }

    #[test]
    fn a_name_that_is_not_a_party_is_refused_naming_the_two() {
        let refused = PartySource
            .read(&resolved(), "carrier")
            .expect_err("refused");
        assert_eq!(refused.technology, "party");
        assert_eq!(refused.property, "carrier");
        assert!(refused.reason.contains("sender and receiver"));

        let bytes =
            message(MessageContext::new().with_value(SENDER_KEY, ContextValue::Binary(vec![7])));
        let refused = PartySource.read(&bytes, "sender").expect_err("bytes");
        assert!(refused.reason.contains("xmip.party holds 1 bytes"));
    }

    #[test]
    fn the_technology_is_party_and_promote_reads_the_prefixed_property() {
        assert_eq!(PartySource.technology(), "party");

        let sources: [&dyn Source; 1] = [&PartySource];
        let promoted = route::promote(&resolved(), &sources, &["party:sender", "party:receiver"])
            .expect("readable");

        assert_eq!(
            promoted.get("party:sender"),
            Some(PartyId::new(42).to_string().as_str())
        );
        assert!(
            Predicate::equals("party:receiver", Value::Text("partner-x".into()))
                .test(&promoted)
                .passed()
        );
    }
}
