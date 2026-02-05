//! Internationalization (i18n) module using Fluent.
//!
//! Provides type-safe localization for error messages and UI text.
//! Supports English, Portuguese (BR), French, and Italian.

use fluent::{FluentArgs, FluentResource};
use fluent_bundle::concurrent::FluentBundle as ConcurrentFluentBundle;
use once_cell::sync::Lazy;
use rust_embed::Embed;
use std::sync::RwLock;
use unic_langid::LanguageIdentifier;

/// Embed all FTL files from the locales directory at compile time
#[derive(Embed)]
#[folder = "locales/"]
struct Locales;

/// Supported locales with fallback order
const SUPPORTED_LOCALES: &[&str] = &["en-US", "pt-BR", "fr", "it"];
const DEFAULT_LOCALE: &str = "en-US";

/// Type alias for thread-safe FluentBundle
type SafeFluentBundle = ConcurrentFluentBundle<FluentResource>;

/// Global localization service
static I18N: Lazy<RwLock<LocalizationService>> =
    Lazy::new(|| RwLock::new(LocalizationService::new(DEFAULT_LOCALE)));

/// Localization service that manages FluentBundle instances
pub struct LocalizationService {
    bundle: SafeFluentBundle,
    #[allow(dead_code)] // Reserved for future use (e.g., locale switching)
    current_locale: String,
}

impl LocalizationService {
    /// Create a new localization service with the specified locale
    pub fn new(locale: &str) -> Self {
        let bundle = Self::create_bundle(locale);
        LocalizationService {
            bundle,
            current_locale: locale.to_string(),
        }
    }

    /// Create a FluentBundle for the given locale with English fallback
    fn create_bundle(locale: &str) -> SafeFluentBundle {
        let lang_id: LanguageIdentifier = locale
            .parse()
            .unwrap_or_else(|_| DEFAULT_LOCALE.parse().unwrap());

        let mut bundle = SafeFluentBundle::new_concurrent(vec![lang_id]);

        // Try to load the requested locale
        if let Some(resource) = Self::load_resource(locale) {
            let _ = bundle.add_resource(resource);
        }

        // Add English as fallback (if not already English)
        if locale != DEFAULT_LOCALE {
            if let Some(fallback) = Self::load_resource(DEFAULT_LOCALE) {
                let _ = bundle.add_resource(fallback);
            }
        }

        bundle
    }

    /// Load FTL resource from embedded files
    fn load_resource(locale: &str) -> Option<FluentResource> {
        let path = format!("{}/main.ftl", locale);
        let content = Locales::get(&path)?;
        let source = std::str::from_utf8(content.data.as_ref()).ok()?;
        FluentResource::try_new(source.to_string()).ok()
    }

    /// Get a localized message by key
    pub fn get(&self, key: &str) -> String {
        self.get_with_args(key, None)
    }

    /// Get a localized message with arguments
    pub fn get_with_args(&self, key: &str, args: Option<&FluentArgs>) -> String {
        if let Some(msg) = self.bundle.get_message(key) {
            if let Some(pattern) = msg.value() {
                let mut errors = vec![];
                let result = self.bundle.format_pattern(pattern, args, &mut errors);
                return result.into_owned();
            }
        }
        // Fallback: return the key itself
        key.to_string()
    }
}

/// Initialize the global locale (call from Android with device locale)
pub fn init_locale(locale: &str) {
    // Normalize locale (e.g., "pt_BR" -> "pt-BR")
    let normalized = locale.replace('_', "-");

    // Find best matching locale
    let matched = SUPPORTED_LOCALES
        .iter()
        .find(|&&l| normalized.starts_with(&l[..2]))
        .unwrap_or(&DEFAULT_LOCALE);

    if let Ok(mut service) = I18N.write() {
        *service = LocalizationService::new(matched);
    }
}

/// Get a localized string by key
pub fn t(key: &str) -> String {
    if let Ok(service) = I18N.read() {
        service.get(key)
    } else {
        key.to_string()
    }
}

/// Get a localized string with a single named argument
pub fn t_with(key: &str, arg_name: &str, arg_value: &str) -> String {
    if let Ok(service) = I18N.read() {
        let mut args = FluentArgs::new();
        args.set(arg_name, arg_value);
        service.get_with_args(key, Some(&args))
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_default() {
        init_locale("en-US");
        assert_eq!(t("error-division-by-zero"), "Cannot divide by zero");
    }

    #[test]
    fn test_portuguese() {
        init_locale("pt-BR");
        assert_eq!(
            t("error-division-by-zero"),
            "Não é possível dividir por zero"
        );
    }

    #[test]
    fn test_with_args() {
        init_locale("en-US");
        let msg = t_with("error-undefined-variable", "name", "x");
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_fallback_to_english() {
        init_locale("unknown-locale");
        // Should fall back to English
        assert_eq!(t("error-division-by-zero"), "Cannot divide by zero");
    }
}
