# Verification Gate Skill Design

**Goal:** 모든 코드 검증을 일관된 verifier 규칙으로 수행하고, unresolved low finding도 남기지 않도록 강제하는 개인 스킬을 만든다.

**Why:** 현재 검증 기준이 대화 맥락에 의존한다. 스킬로 고정하면 Claude Code나 다른 세션에 verifier 역할을 이양할 때도 같은 판정 기준을 재사용할 수 있다.

**Design:**
- 단일 스킬 이름은 `verification-gate`
- 기본 역할은 `independent verifier`, not implementer
- fresh verification evidence 없이는 어떤 성공 주장도 허용하지 않음
- `High/Medium/Low` findings를 모두 보고하되, `Low`만 남아도 `PROCEED`를 금지
- 최종 의사결정은 `BLOCKED`, `CHANGES_REQUIRED`, `CLEANUP_REQUIRED`, `PROCEED`

**Bundled resources:**
- `references/claude-code-verifier-prompt.md`
  - Claude Code에 verifier 역할을 이양할 때 그대로 붙여넣을 프롬프트

**Non-goals:**
- 일반 코드 리뷰 스킬 대체
- 구현 자동화
- 브레인스토밍/설계 검토

**Trigger examples:**
- "이 커밋 다음 task 가도 되나?"
- "검증만 해줘"
- "테스트 통과했다는데 진짜 맞는지 봐줘"
- "완료 주장 전에 fresh verification 돌려줘"

