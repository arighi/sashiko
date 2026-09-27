# Validation model prototype

## Goal

Allow the concern-validation stages to use a separately configured AI provider.
The validation provider may have a different model, endpoint, and credential
from the main provider. When absent, all requests use the existing main
provider.

## Configuration

`[ai.validation]` contains a complete second AI configuration with `provider`
and `model`. For an OpenAI-compatible endpoint,
`[ai.validation.openai_compat]` supplies `base_url` and `api_key_env`.
`api_key_env` names an environment variable containing the credential. The
main `[ai.openai_compat]` also accepts `api_key_env`, allowing two independent
credentials without putting either one in Settings.toml. If omitted, the
existing `OPENAI_API_KEY` then `LLM_API_KEY` lookup applies.

The nested provider inherits only normal per-field defaults. It does not
inherit the main endpoint or credential implicitly, because accidentally
sending a credential to the wrong endpoint is unsafe. Operators can set the
same values explicitly when desired.

## Routing

AI requests carry a serialized `model_role` with `Main` as the default for
backward compatibility. Verification and every parallel post-verification
stage mark their requests as `Validation`; all other stages use `Main`.
Verification consolidates concerns and dismissals, then post-verification
resolves hard cases, so both stages require the same validation model. A shared routing provider sends requests to the
selected provider. If no validation provider is configured, the router sends
both roles to the main provider.

The local review creates both providers and routes directly. The daemon
creates both providers; a worker sends the role over the existing stdio AI
protocol and the daemon routes before invoking the provider. The daemon's
normal concurrency, retry, quota, accounting, and cache wrappers still apply
to each request. The selected provider's cache identity distinguishes its
responses; the role itself is excluded from cache keys so existing entries
remain reusable when no validation provider is configured.

## Scope and verification

This routing applies to verification and post-verification in both Linux
and Sashiko patch-review workflows. It does not change bug-analysis workflows,
discovery, report generation, or prompt content. Tests cover
configuration, credential selection, stage roles, routing, stdio role
serialization, and default behavior. Run `make check-pr` before committing.
