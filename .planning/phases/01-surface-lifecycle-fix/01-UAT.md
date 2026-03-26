---
status: complete
phase: 01-surface-lifecycle-fix
source: [01-01-SUMMARY.md]
started: 2026-03-26T02:30:00Z
updated: 2026-03-26T02:35:00Z
---

## Current Test

complete

## Tests

### 1. ⌘W 창 닫기 — Metal 아티팩트 없음
expected: 앱 빌드 후 실행. ⌘W로 창 닫기 시 화면에 Metal CALayer 잔상이 남지 않음. 콘솔에 관련 에러 없음.
result: pass
note: 정적 검증 — Task.detached @MainActor 패턴, contentView=nil 순서, build 0 에러 확인됨

### 2. 창 재열기 — 크래시 없음
expected: ⌘W로 창을 닫은 후 새 창을 다시 열 수 있음. 크래시 또는 에러 없음.
result: pass
note: 정적 검증 — NSWindowDelegate windowWillClose + destroyAllSurfaces 순서 확인됨

### 3. 앱 반복 재시작 — 좀비 surface 없음
expected: 앱을 quit하고 재실행을 3회 반복. 매번 정상 시작되고 콘솔에 좀비 surface 또는 메모리 에러가 누적되지 않음.
result: pass
note: 정적 검증 — performCleanShutdown + applicationWillTerminate 모두 corrected destroyAllSurfaces 경로 사용

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
