use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomId {
    pub base: String,
    pub name: String,
    pub data: Vec<String>,
}

impl CustomId {
    pub const fn new_with(base: String, name: String, data: Vec<String>) -> Self {
        Self { base, name, data }
    }
    pub const fn new(base: String, name: String) -> Self {
        Self::new_with(base, name, vec![])
    }

    pub fn append(&mut self, data: impl Into<String>) -> Result<()> {
        let string = data.into();
        let length = self.to_string().len() + string.len() + 1;

        if length > 64 {
            return err_wrap!("maximum custom identifier length exceeded ({length} / 64)");
        }

        self.data.push(string);
        Ok(())
    }
}

impl FromStr for CustomId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(';');
        let Some([base, name]) = parts.next().and_then(|s| s.split('_').array_chunks().next()) else {
            return err_wrap!("invalid custom identifier header");
        };
        let data = parts.map(str::to_string).collect();

        Ok(Self::new_with(base.to_string(), name.to_string(), data))
    }
}

impl From<CustomId> for String {
    fn from(value: CustomId) -> Self {
        value.to_string()
    }
}

impl From<&CustomId> for String {
    fn from(value: &CustomId) -> Self {
        value.to_string()
    }
}

impl Display for CustomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { base, name, data } = self;

        if data.is_empty() {
            write!(f, "{base}_{name}")
        } else {
            let data = data.join(";");

            write!(f, "{base}_{name};{data}")
        }
    }
}
