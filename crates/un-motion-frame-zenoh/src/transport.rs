#[cfg(feature = "zenoh-transport")]
use zenoh::Config;

use crate::Error;

/// Zenoh session の接続経路だけを上位アプリから指定するための設定。
///
/// `Default` は `zenoh::Config::default()` を一切変更しない。従来の同一PC接続や
/// multicast scouting の挙動を保ったまま、必要なアプリだけが固定 listen endpoint
/// または明示 connect endpoint を追加できる。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZenohSessionConfig {
	listen_endpoints: Option<Vec<String>>,
	connect_endpoints: Option<Vec<String>>,
	multicast_scouting: Option<bool>,
}

impl ZenohSessionConfig {
	pub fn with_listen_endpoint(mut self, endpoint: impl Into<String>) -> Self {
		self.listen_endpoints.get_or_insert_with(Vec::new).push(endpoint.into());
		self
	}

	pub fn with_connect_endpoint(mut self, endpoint: impl Into<String>) -> Self {
		self.connect_endpoints.get_or_insert_with(Vec::new).push(endpoint.into());
		self
	}

	pub fn with_multicast_scouting(mut self, enabled: bool) -> Self {
		self.multicast_scouting = Some(enabled);
		self
	}

	pub fn listen_endpoints(&self) -> Option<&[String]> {
		self.listen_endpoints.as_deref()
	}

	pub fn connect_endpoints(&self) -> Option<&[String]> {
		self.connect_endpoints.as_deref()
	}

	pub fn multicast_scouting(&self) -> Option<bool> {
		self.multicast_scouting
	}

	#[cfg(feature = "zenoh-transport")]
	pub(crate) fn to_zenoh_config(&self) -> Result<Config, Error> {
		let mut config = Config::default();

		if let Some(endpoints) = &self.listen_endpoints {
			config
				.insert_json5("listen/endpoints", &serialize_endpoints(endpoints, "listen")?)
				.map_err(|error| Error::transport(format!("invalid Zenoh listen endpoints: {error}")))?;
		}
		if let Some(endpoints) = &self.connect_endpoints {
			config
				.insert_json5("connect/endpoints", &serialize_endpoints(endpoints, "connect")?)
				.map_err(|error| Error::transport(format!("invalid Zenoh connect endpoints: {error}")))?;
		}
		if let Some(enabled) = self.multicast_scouting {
			config
				.insert_json5("scouting/multicast/enabled", if enabled { "true" } else { "false" })
				.map_err(|error| Error::transport(format!("invalid Zenoh multicast scouting setting: {error}")))?;
		}

		Ok(config)
	}
}

#[cfg(feature = "zenoh-transport")]
fn serialize_endpoints(endpoints: &[String], purpose: &str) -> Result<String, Error> {
	serde_json::to_string(endpoints).map_err(|error| Error::transport(format!("failed to serialize Zenoh {purpose} endpoints: {error}")))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_preserves_all_zenoh_defaults() {
		let config = ZenohSessionConfig::default();
		assert_eq!(config.listen_endpoints(), None);
		assert_eq!(config.connect_endpoints(), None);
		assert_eq!(config.multicast_scouting(), None);
	}

	#[test]
	fn builder_keeps_explicit_connection_policy() {
		let config = ZenohSessionConfig::default()
			.with_listen_endpoint("tcp/0.0.0.0:39542")
			.with_connect_endpoint("tcp/192.0.2.10:39542")
			.with_multicast_scouting(false);

		assert_eq!(config.listen_endpoints(), Some(["tcp/0.0.0.0:39542".to_string()].as_slice()));
		assert_eq!(config.connect_endpoints(), Some(["tcp/192.0.2.10:39542".to_string()].as_slice()));
		assert_eq!(config.multicast_scouting(), Some(false));
	}

	#[cfg(feature = "zenoh-transport")]
	#[test]
	fn invalid_endpoint_is_rejected_before_opening_session() {
		let error = ZenohSessionConfig::default()
			.with_connect_endpoint("not an endpoint")
			.to_zenoh_config()
			.expect_err("invalid endpoint");
		assert!(error.to_string().contains("invalid Zenoh connect endpoints"));
	}
}
