@0xe1cbad6bb14d94da;

enum Role {
  ui @0;
  core @1;
}

enum Compression {
  none @0;
}

struct Version {
  major @0 :UInt16;
  minor @1 :UInt16;
}

struct Hello {
  supportedVersions @0 :List(Version);
  role @1 :Role;
  maxPayload @2 :UInt32;
  compression @3 :Compression;
}

struct HelloAck {
  selectedVersion @0 :Version;
  role @1 :Role;
  maxPayload @2 :UInt32;
  compression @3 :Compression;
}

enum ErrorCode {
  incompatibleMajor @0;
  malformedRequest @1;
  unknownMethod @2;
  invalidArgument @3;
  internal @4;
}

struct Error {
  code @0 :ErrorCode;
  message @1 :Text;
}

struct EditRequest {
  traceId @0 :UInt64;
  expectedVersion @1 :UInt64;
  charOffset @2 :UInt64;
  text @3 :Text;
  dispatchNs @4 :UInt64;
}

struct ViewportSnapshot {
  traceId @0 :UInt64;
  version @1 :UInt64;
  text @2 :Text;
  coreReceivedNs @3 :UInt64;
  versionIncrementNs @4 :UInt64;
  viewportEmitNs @5 :UInt64;
}

struct Heartbeat {
  sequence @0 :UInt64;
}
