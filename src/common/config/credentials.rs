use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

pub struct Credentials<'a> {
    username: Cow<'a, str>,
    password: Option<Cow<'a, str>>,
}

impl<'a> Credentials<'a> {
    pub(super) fn new<P: Into<Cow<'a, str>>>(
        username: impl Into<Cow<'a, str>>,
        password: Option<P>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.map(Into::into),
        }
    }

    #[allow(unused)]
    pub(super) fn username(&self) -> &str {
        &self.username
    }

    #[allow(unused)]
    pub(super) fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl Display for Credentials<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { username, password } = self;
        f.write_str(username)?;
        if let Some(password) = password {
            f.write_char(':')?;
            f.write_str(password)?;
        }
        Ok(())
    }
}
