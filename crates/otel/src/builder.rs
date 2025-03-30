use std::fmt::Display;
use crate::otel::{init_tracing_subscriber, Config, OtelGuard};
use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::attribute::DEPLOYMENT_ENVIRONMENT_NAME;
use opentelemetry_semantic_conventions::attribute::SERVICE_VERSION;

#[derive(Debug, Clone)]
pub enum Environment {
    Dev,
    Prod,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Environment::Dev => write!(f, "dev"),
            Environment::Prod => write!(f, "prod"),
        }
    }
}

#[derive(Debug, Default)]
pub struct OtelBuilder {
    attributes: Vec<KeyValue>,
    headers: Option<std::collections::HashMap<String, String>>,
    project_name: Option<String>,
    environment: Option<Environment>,
}

impl OtelBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            headers: None,
            project_name: None,
            environment: None,
        }
    }

    pub fn set_token(mut self, token: String) -> Self {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".into(), token);
        self.headers = Some(headers);
        self
    }

    pub fn set_project_name(mut self, name: String) -> Self {
        self.project_name = Some(name);
        self
    }

    pub fn set_version(mut self, version: String) -> Self {
        self.attributes
            .push(KeyValue::new(SERVICE_VERSION, version));
        self
    }

    #[must_use]
    pub fn set_environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment.clone());
        self.attributes.push(KeyValue::new(
            DEPLOYMENT_ENVIRONMENT_NAME,
            environment.to_string(),
        ));
        self
    }

    /// Can be used to set extra `attributes`
    /// to your telemetry. E.g. `KeyValue::new(SERVICE_NAME, "my-service")`
    pub fn set_attributes(mut self, attributes: &mut Vec<KeyValue>) -> Self {
        self.attributes.append(attributes);
        self
    }

    /// It's important that the guard is not dropped and must be assigned to a named variable
    ///     let _guard = OtelBuilder::new()
    //         .set_environment("dev".to_string())
    //         .set_token("<my_token>".to_string())
    //         .set_project_name(env!("CARGO_PKG_NAME").to_string())
    //         .set_version(env!("CARGO_PKG_VERSION").to_string())
    //         .build()
    //         .unwrap();
    ///
    pub fn build(self) -> Result<OtelGuard, Box<dyn std::error::Error>> {
        self.environment.ok_or("Environment not set")?;
        let otel = Config {
            headers: self.headers.ok_or("Header not set")?,
            project_name: self.project_name.ok_or("Project name not set")?,
            attributes: self.attributes,
        };
        Ok(init_tracing_subscriber(otel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

    #[test]
    fn test_ok() {
        let guard = OtelBuilder::new()
            .set_environment(Environment::Dev)
            .set_token("token".to_string())
            .set_project_name("project".to_string())
            .build()
            .unwrap();
    }

    #[test]
    fn test_custom_attributes() {
        let guard = OtelBuilder::new()
            .set_environment(Environment::Dev)
            .set_token("token".to_string())
            .set_project_name("project".to_string())
            .set_attributes(&mut vec![KeyValue::new(SERVICE_NAME, "test-service")]);
        assert_eq!(
            guard.attributes,
            vec![
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "dev"),
                KeyValue::new(SERVICE_NAME, "test-service")
            ]
        );
    }

    #[test]
    fn test_missing() {
        // env
        assert!(OtelBuilder::new()
            .set_token("token".to_string())
            .set_project_name("project".to_string())
            .set_attributes(&mut vec![KeyValue::new(SERVICE_NAME, "test-service")])
            .build()
            .is_err());
        // token
        assert!(OtelBuilder::new()
            .set_environment(Environment::Dev)
            .set_project_name("project".to_string())
            .set_attributes(&mut vec![KeyValue::new(SERVICE_NAME, "test-service")])
            .build()
            .is_err());
        // project name
        assert!(OtelBuilder::new()
            .set_token("token".to_string())
            .set_environment(Environment::Dev)
            .set_attributes(&mut vec![KeyValue::new(SERVICE_NAME, "test-service")])
            .build()
            .is_err());
    }
}
