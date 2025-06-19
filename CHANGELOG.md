# Changelog

## [0.1.8] - 2025-06-19

### Fixed
- Fixed panic in `APIVersion::from_u64()` when encountering unsupported version numbers
  - Now returns proper error instead of panicking
  - Prevents server crashes when loading contracts with invalid version data
  - Critical fix for River invitation bug where requests would hang indefinitely

### Changed
- `APIVersion::from_u64()` now returns `Result<Self, VersionError>` instead of `Self`
- Added `VersionError` enum for better error handling

## [0.1.7] - Previous release