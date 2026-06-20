// Placeholder module; concrete Tonic service implementations live here once
// the Lore .proto files are vendored and compiled via build.rs.
//
// Planned services (mirroring the Lore gRPC surface):
//   - RepoService   — branch CRUD, revision read/write
//   - CasService    — chunk find-missing, upload, download
//   - LockService   — acquire / release / query file locks
//   - AdminService  — tenant-scoped admin operations
