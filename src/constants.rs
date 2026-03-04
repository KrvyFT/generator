pub const DEFAULT_SUPPORT_EMAIL: &str = "krvyft@pm.me";
pub const D1_BINDING: &str = "DB";
pub const KV_BINDING: &str = "KV_LIMITER";

/// AES-256-GCM key for encrypting frontend scripts (32 bytes, hex-encoded).
/// Change this value to rotate the encryption key.
pub const SCRIPT_ENCRYPT_KEY_HEX: &str =
    "9f2e8a1c7b3d6054f8e1c2d9a7b4e0f3621d5c8a3b7e09f4d6a2c8e1b5f7034d";
