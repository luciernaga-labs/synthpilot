use std::fmt;

macro_rules! static_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(&'static str);

        impl $name {
            pub const fn new(value: &'static str) -> Self {
                Self(value)
            }

            pub const fn as_str(self) -> &'static str {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }
    };
}

static_id!(DeviceId);
static_id!(EngineId);
static_id!(ParameterId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_expose_their_text() {
        let id = ParameterId::new("awm2.element-level");
        assert_eq!(id.as_str(), "awm2.element-level");
        assert_eq!(id.to_string(), "awm2.element-level");
        assert_eq!(DeviceId::new("a").as_str(), "a");
        assert_eq!(EngineId::new("b").to_string(), "b");
    }
}
