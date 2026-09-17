# TypeSafe AI, System One models, and Jev: a research report for Warrant

Status: research report, 2026-09-16. Author: Fable (research lane), for Warrant. Sources fetched live on 2026-09-16.

Scope note: Warrant is the rebuild of Specgate. The vision document this report is grounded in still uses the Specgate name, and quotations from it keep that name. Every external fact below carries its source URL and an access date. Where a statement is my inference rather than a sourced fact, the sentence says so. Where a number or limit could not be verified from a primary source, it is marked unverified. Quotations are kept short and marked with straight quotes.

## 1. Summary

TypeSafe AI announced its first "System One" model, Jev, on 2026-09-15 (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). Jev is a hosted model that takes one piece of state, text or JSON, plus a map of typed questions, and returns one typed answer per question with probabilities. It does not generate text, code, or explanations (source: https://docs.typesafe.ai/concepts/system-one.md, as of 2026-09-16). The three question types are Noul (yes or no, returns a probability), Choice (one option from up to 255, returns a distribution and a confidence), and Score (an ordered rubric, returns an expected level, a distribution, and a confidence) (sources: https://docs.typesafe.ai/api.md and https://docs.typesafe.ai/primitives/choice.md, as of 2026-09-16). One request is bounded by a shared token budget of "around 32,000 tokens" for state plus questions (source: https://docs.typesafe.ai/primitives.md, as of 2026-09-16). Listed price is $0.042 per million input tokens with output tokens free, and the claimed end-to-end latency is 70 to 500 milliseconds (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). The company raised a $40 million seed round led by DCVC and is offering Jev through an early-access waitlist (source: https://www.businesswire.com/news/home/20260915525333/en/TypeSafe-AI-Emerges-From-Stealth-With-%2440M-in-Funding-With-New-Model-for-Composable-AI, as of 2026-09-16).

What it is not. Jev is not a code reader in the sense a coding agent is, it cannot follow imports or open a second file, and it cannot explain its answer. Its "cannot hallucinate" claim means only that answers stay inside the schema you supplied. TypeSafe's own FAQ says it "can choose the wrong one" (source: https://typesafe.ai/, FAQ "Can Jev still get things wrong?", as of 2026-09-16). On TypeSafe's own workflow evals, Jev's mean accuracy against consensus labels is 67.8 percent, versus 73.1 percent for Claude Opus 5 and 74.1 percent for GPT-5.6 Sol, at roughly one four-hundredth of Opus 5's cost per case (source: https://evals.typesafe.ai/, as of 2026-09-16). No published evidence covers source-code taste tasks. The closest thing is a small third-party lab by Every that ranked 8 synthetic files and screened 10 agent actions (source: https://typesafe-parallel-judgment-lab.every-4573.chatgpt.site/downloads/experiment-report.md, as of 2026-09-16).

Fit verdict, in one paragraph. The request shape (state plus typed questions, calibrated typed answers out) is a good interface for Warrant's judgment lane, and Jev's price makes per-hunk evaluation of a taste profile affordable at any plausible volume. The evidence for accuracy on code is absent, access is waitlisted, and the model is explicitly consistent rather than deterministic, so the judgment lane must be designed so that Jev can be wrong or unavailable without changing a verdict. Adopt the shape, treat Jev as one backend behind it, keep every model output out of the compliance facet, and run the experiments in section 8 before promising anything about taste.

## 2. The company and the claim

TypeSafe AI, Inc. is a San Francisco company founded in 2024 by Diogo Almeida (CEO), Erik Gafni, and Sasha Sheng. On 2026-09-15 it announced that it had "emerged from stealth with $40 million in seed funding led by DCVC" (source: https://www.businesswire.com/news/home/20260915525333/en/TypeSafe-AI-Emerges-From-Stealth-With-%2440M-in-Funding-With-New-Model-for-Composable-AI, as of 2026-09-16). The Register reported the same figure and described Almeida as a former OpenAI researcher and co-inventor of RLHF (source: https://www.theregister.com/ai-and-ml/2026/09/16/typesafe-ai-debuts-model-for-machines-that-plays-doom/5296711, as of 2026-09-16). TypeSafe's team page says Almeida "co-invented RLHF and InstructGPT" and was previously at Google Brain, and that the team works in person five days a week in San Francisco (source: https://typesafe.ai/team, as of 2026-09-16). The privacy policy identifies the legal entity as Typesafe AI, Inc. and says the services are hosted in the United States (source: https://typesafe.ai/legal/privacy-policy, as of 2026-09-16).

The manifesto's thesis is that intelligence is already sufficient and the bottleneck is composability: "the bottleneck isn't raw intelligence. It's that today's intelligence is hard to build on." The manifesto frames chat models as horseless carriages, argues that software needs AI "as a primitive that any programmer can invoke for semantic judgement and decisions," and closes with the slogan "We're building prod, not God." (source: https://typesafe.ai/manifesto, as of 2026-09-16). Two supporting posts precede the launch. "The Bitterest Lesson" (2026-09-10) argues that "doing the right task > data > compute > algorithms" and that RLHF succeeded because it changed the task, not the scale (source: https://typesafe.ai/blog/the-bitterest-lesson, as of 2026-09-16). "Lies, Damned Lies, and Benchmarks" (2026-09-11) commits the company to "no standard benchmark table in our model releases," dated eval snapshots that are retired once posted, and publishing "evidence that looks bad for us" (source: https://typesafe.ai/blog/antibenchmaxxing, as of 2026-09-16).

The technical claim is a third post-training path. The docs describe RLHF as the method that "turned pretrained models into chatbots," RLVR as the method that "created reasoning models that are strong at tasks such as mathematics, but slower and more expensive," and RLCD, Reinforcement Learning for Calibrated Decisions, as training that "returns decisions and calibrated probabilities instead of generated text" (source: https://docs.typesafe.ai/introduction/machine-learning-primer.md, as of 2026-09-16). The announcement adds "a new model architecture, parallel sampler" and says Jev "outputs all probabilities in parallel instead of autoregressively generating by token" (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). The FAQ's answer to "Is Jev just a smaller LLM?" is "Jev is neither small nor an LLM" (source: https://typesafe.ai/, as of 2026-09-16). No architecture paper, parameter count, or weights are published as of this date. The speed and cost figures are observable. The mechanism is asserted.

Calibration is the load-bearing promise. The primer states the target directly: across many predictions, "Outcomes assigned a probability of 0.8 should occur about 80% of the time," and then qualifies it: "These rates describe groups of predictions, not a guarantee about any single answer" (source: https://docs.typesafe.ai/introduction/machine-learning-primer.md, as of 2026-09-16). On training data, the FAQ says "We make all the data ourselves. We wouldn't train on your data even if you asked us to" (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, FAQ, as of 2026-09-16).

## 3. The API contract, precisely

The API surface is two endpoints. `POST https://api.typesafe.ai/v1/systemone` evaluates one state against a map of questions, and `GET /v1/models` lists the models and aliases available to the account. Authentication is a bearer API key in the Authorization header. The published OpenAPI document reports API version 0.2.0 (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16). The v1 endpoint replaced a preview endpoint with a different request shape, so third-party write-ups that mention a `prompts` array or `document` field describe the retired preview (source: https://docs.typesafe.ai/migrating-to-v1.md, as of 2026-09-16).

Request shape. The body has three required fields. `state` is a string, a JSON object, or a JSON array. `model` is a model name or alias, with `jev-latest` as the documented flagship alias and the SDK default. `questions` is a map with at least one entry, keyed by ids you choose. The key "is not sent to the underlying model and is not used in inference" (sources: https://api.typesafe.ai/openapi.json and https://docs.typesafe.ai/api.md, as of 2026-09-16). The docs recommend an object for state so each part has a name, and recommend referring to nested parts from a question with a backticked dot path such as `ticket.messages[0].text` (sources: https://docs.typesafe.ai/concepts/state.md and https://docs.typesafe.ai/primitives.md, as of 2026-09-16).

The three primitives and their exact answer fields, from the OpenAPI schema (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16):

- Noul. Request: `type: "noul"`, optional `instructions`, optional `criteria` with optional `true` and `false` descriptions. Answer: `type` and `noul`, a number from 0 to 1 that is the probability of yes or true. A Noul answer carries no confidence field. The docs say a value near 0.5 "gives yes and no similar probability" and warn that 0.5 "does not mean medium skill" (source: https://docs.typesafe.ai/primitives/noul.md, as of 2026-09-16).
- Choice. Request: `type: "choice"`, optional `instructions`, required `criteria`, a map from option name to a description or null. Answer: `type`, `choice` (the highest-probability option), `confidence` from 0 to 1, and `probabilities`, a map from every option to a probability that sums to approximately 1.
- Score. Request: `type: "score"`, optional `instructions`, required `criteria`, an ordered array of level descriptions whose position is the level number starting at zero. The OpenAPI schema says `minItems: 1`, the HTTP docs say "at least two levels," and the JS SDK types require two. Answer: `type`, `score` (the probability-weighted mean of the level numbers, which can fall between levels), `confidence`, `legend` (level number to description), and `probabilities` keyed by level number as a string.

Structured instructions and criteria. Every `instructions` field and every criteria description accepts a string, a JSON object, a JSON array, or null. The SDKs name this union `EntryType` in JavaScript and `JSONContent` in Python (sources: https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-js/HEAD/src/types.ts and https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_core/question_types.py, as of 2026-09-16). The advanced page uses this to share one `field` object across several questions, to attach a JSON rubric that says what each Choice option "does and does not cover," and to walk a taxonomy by passing each node's subtree as the option value (source: https://docs.typesafe.ai/primitives/advanced.md, as of 2026-09-16).

Confidence derivation. For Choice and Score answers, `confidence` is "a statistic computed from the probability distribution the answer already gives you," where a flatter distribution means lower confidence, and TypeSafe explicitly says you are "never locked into our definition" because the full distribution is returned (source: https://docs.typesafe.ai/confidence.md, as of 2026-09-16). The exact formula is not published. The migration guide notes that "The computation behind confidence changed" between preview and v1 and that confidence-based logic "should be carefully re-evaluated" (source: https://docs.typesafe.ai/migrating-to-v1.md, as of 2026-09-16).

Independence of questions. "Every question in a request sees the same state, is evaluated independently," and "one answer does not become context for another question." A second request is warranted only when code cannot build it until it has the first answer (source: https://docs.typesafe.ai/primitives.md, as of 2026-09-16). The parallel-questions cookbook reports that batching 13 questions into one call versus 13 calls produced the same answers with the same run-to-run variance and was 12.2 times cheaper and 10.0 times faster on a 54,000-character document (source: https://docs.typesafe.ai/cookbooks/parallel_questions.md, as of 2026-09-16).

Limits. The token budget is shared by state and questions and is "around 32,000 tokens, roughly 150,000 characters of English text" (source: https://docs.typesafe.ai/primitives.md, as of 2026-09-16). A Choice question "accepts up to 255 options," and the announcement says that for higher cardinality TypeSafe uses "a 2 stage-system of scoring independently then making an explicit choice" (sources: https://docs.typesafe.ai/primitives/choice.md and https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). For Score, the guidance is "Use as many levels as you can describe distinctly, up to 10" (source: https://docs.typesafe.ai/primitives/score.md, as of 2026-09-16). The OpenAPI schema declares no maximum on the number of questions, no maximum state size, and no rate limit numbers (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16). What happens when the budget is exceeded is unverified. No tokenizer is published, so Warrant cannot count tokens locally before sending. The response reports `usage.input_tokens` and `usage.output_tokens` after the fact, and the schema notes output tokens "are currently free of charge" (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16).

Response `model` field. The schema says the returned model name "May differ from the alias supplied in the request" (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16). This is the field Warrant should record in a receipt, not the alias it sent.

Error classes. The HTTP docs list 401 for a missing or invalid key, 422 for validation failures with a body naming the offending field, 429 for rate limiting, and 529 for overload, and recommend exponential backoff for 429 and 529 (source: https://docs.typesafe.ai/api.md, as of 2026-09-16). A live probe of `GET /v1/models` with no key returned HTTP 403, not 401, with the body `{"detail":{"error_type":"authentication_error","message":"Must supply an API key! ..."}}` and an `x-typesafe-request-id` header (my own request, as of 2026-09-16). The Python SDK maps 400, 401, 403, 404, 422, and 429 to distinct exception classes, treats any other 5xx as an internal server error, and exposes `retry_after_ms` on rate-limit errors and `request_id` on all API errors (source: https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_core/errors.py, as of 2026-09-16).

Retries and SDK defaults. Both SDKs retry twice by default with exponential backoff starting at 500 milliseconds, capped at 5 seconds, with 25 percent jitter, on statuses 408, 429, and 500 through 599, and both honor `Retry-After` and `retry-after-ms`. The JS per-attempt timeout default is 10 seconds with no total retry budget. The Python default per-attempt timeout is 30 seconds with a total retry time budget (sources: https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-js/HEAD/src/types.ts and https://docs.typesafe.ai/sdk/python/api/retries.md, as of 2026-09-16). SDK versions as of this date are 0.6.0 for both Python and JavaScript, released 2026-09-15, following initial public releases 0.5.7 on 2026-09-14 (Python) and 2026-09-11 (JavaScript). The 0.6.0 release is a breaking change that turned Score criteria from an integer-keyed dictionary into an ordered sequence (sources: https://docs.typesafe.ai/sdk/python/changelog.md and https://docs.typesafe.ai/sdk/javascript/changelog.md, as of 2026-09-16).

Model ids. `GET /v1/models` returns objects with `name`, `description`, and `release_date`, and the schema example is `jev-latest` with release date 2026-09-15 (source: https://api.typesafe.ai/openapi.json, as of 2026-09-16). Two cookbooks pin `jev-1.12` as the model id used for their recorded runs (sources: https://docs.typesafe.ai/cookbooks/parallel_questions.md and https://docs.typesafe.ai/cookbooks/sde_cascade.md, as of 2026-09-16), and the hierarchical classification cookbook uses the same id (source: https://docs.typesafe.ai/cookbooks/hierarchical_classification.md, as of 2026-09-16). Every's pre-launch lab, generated 2026-08-28, used a model id `speed_latest` that appears nowhere in the current docs (source: https://typesafe-parallel-judgment-lab.every-4573.chatgpt.site/downloads/experiment-report.md, as of 2026-09-16). Whether every account can address `jev-1.12` directly, and how long a pinned id stays served, is unverified.

A compact example, in the exact v1 shape (request fields from https://docs.typesafe.ai/api.md, as of 2026-09-16, values illustrative):

```json
{
  "state": {
    "hunk": "@@ -12,6 +12,14 @@ ... +const cache = new Map() ...",
    "contract": {"id": "caching-owner", "intent": "actions/cache owns all memoization"},
    "map": {"existing_owner": "actions/cache", "consumers": 4}
  },
  "model": "jev-latest",
  "questions": {
    "introduces_second_owner": {
      "type": "noul",
      "instructions": "Does `hunk` create a second implementation of the responsibility named in `contract.intent`?",
      "criteria": {"true": "A new cache, registry, or dispatcher for the same responsibility", "false": "Reuses or extends `map.existing_owner`"}
    },
    "overlap_kind": {
      "type": "choice",
      "instructions": "How does `hunk` overlap with `map.existing_owner`?",
      "criteria": {"behavioral": "Same behavior", "structural": "Same shape, different behavior", "linguistic": "Only the names match", "none": null}
    },
    "depth": {
      "type": "score",
      "instructions": "How much behavior does the new interface in `hunk` hide from its callers?",
      "criteria": ["Pass-through wrapper", "Some policy hidden", "Substantial behavior behind a small interface"]
    }
  }
}
```

A response in the documented shape (field names from https://docs.typesafe.ai/api.md, as of 2026-09-16, values illustrative):

```json
{
  "model": "jev-1.12",
  "answers": {
    "introduces_second_owner": {"type": "noul", "noul": 0.91},
    "overlap_kind": {"type": "choice", "choice": "behavioral", "confidence": 0.83,
                     "probabilities": {"behavioral": 0.86, "structural": 0.09, "linguistic": 0.04, "none": 0.01}},
    "depth": {"type": "score", "score": 0.4, "confidence": 0.71,
              "legend": {"0": "Pass-through wrapper", "1": "Some policy hidden", "2": "Substantial behavior behind a small interface"},
              "probabilities": {"0": 0.65, "1": 0.30, "2": 0.05}}
  },
  "usage": {"input_tokens": 2140, "output_tokens": 9}
}
```

## 4. Design patterns TypeSafe documents that matter for a code-quality profile

Speculative fan-out. Put every question the code might need into one request, including questions whose answers only matter on some branches, and let code discard the irrelevant ones. "All questions are evaluated in parallel, so adding more questions to a call typically doesn't add any latency." For Warrant this means a taste profile can ask its whole battery per hunk in one call rather than sequencing rules (source: https://docs.typesafe.ai/patterns/fan-out.md, as of 2026-09-16).

Composite scoring. Break a judgment into atomic Score dimensions, normalize each to 0 to 1, and combine with weights that live in code, so "you can adjust the weights to find the right balance" without re-prompting. The relevance for Warrant is that the profile's weights and thresholds become reviewable constants in the profile file, which is also what TypeSafe's agent skill recommends: put "the questions and thresholds in a single place so they're easy to review" (sources: https://docs.typesafe.ai/patterns/composite-scoring.md and https://docs.typesafe.ai/agent-skill.md, as of 2026-09-16).

Confidence-gated routing. Treat confidence as a second axis: a global floor below which the model is "genuinely uncertain" and the case routes to a person, then per-action thresholds that rise with the stakes, with the worked example gating a balance read at 0.6 and a transfer at above 0.85 (source: https://docs.typesafe.ai/patterns/confidence-routing.md, as of 2026-09-16). The confidence page adds the three-band model: act, proceed with caution, do not act (source: https://docs.typesafe.ai/confidence.md, as of 2026-09-16).

Guardrail batteries with named threshold policies. The LLM guardrails cookbook screens each message with one request carrying a battery of hazard Nouls plus one severity Score, then routes in code with two thresholds per hazard, a review threshold and an action threshold, and a severity cutoff that upgrades review to block. The thresholds are grouped into named policies, `strict` (review 0.35, action 0.70, severity block 2.0) and `permissive` (action 0.85), and a precedence list decides the outcome when several fire. "A policy is just those numbers under a name" (source: https://docs.typesafe.ai/cookbooks/llm_guardrails.md, as of 2026-09-16). This is the closest published analog to a Warrant taste profile.

Citation checks. A string match first catches fabricated quotes, then one Choice question reads the surrounding context and returns supports, contradicts, or says nothing, with a confidence gate at 0.8 that sends low-confidence verdicts to a human. In the worked example the four accurate citations came back verified at confidence 0.93 or higher and all four planted failures were caught (source: https://docs.typesafe.ai/cookbooks/citation_check.md, as of 2026-09-16). Intent matching in Warrant has the same shape: deterministic evidence first, then a typed judgment about whether the evidence supports the claim.

Hierarchical classification, including a codebase tree. The cookbook walks deep taxonomies (patent classes, a retail taxonomy, MeSH, and "TypeSafe's cookbook repository hierarchy, searched from folders to source files") by asking one Choice per level with the node's children as options, and runs a beam search that keeps K paths alive and scores each path by the geometric mean of its edge probabilities. The codebase case is a developer-search query that expects a specific Python file as the leaf (source: https://docs.typesafe.ai/cookbooks/hierarchical_classification.md, as of 2026-09-16). This is the pattern that gets past the 255-option cap for a census over a large module tree.

SDE cascade. Extract with a cheap model, verify each field with per-field Nouls such as "is this value absent from the source?", and escalate to an expensive reasoning model only when a verifier fires. The cookbook argues that schema-following mistakes are not the mistakes to expect from an LLM, and that "constrained decoding doesn't fix the underlying issue" (source: https://docs.typesafe.ai/cookbooks/sde_cascade.md, as of 2026-09-16). Warrant's analog is cheap triage before an expensive explanation.

Self-consistency cookbooks. Two cookbooks repeat one rubric 15 times per model. On 14 Nouls over one insurance claim, TypeSafe's mean per-question probability standard deviation was 0.0102, below every LLM condition, but one answer spanned 0.43 to 0.53 across repeats, crossing a 0.5 threshold, which the cookbook resolves by mapping 0.30 to 0.70 to an explicit `uncertain` outcome (source: https://docs.typesafe.ai/cookbooks/consistency_noul_cookbook.md, as of 2026-09-16). On 8 Choice questions over one moderation case, "TypeSafe flips on 2 of the 8 questions," and requiring a top probability of at least 0.60 raised agreement to 99.2 percent while leaving 74.2 percent of answers automatic (source: https://docs.typesafe.ai/cookbooks/consistency_choice_cookbook.md, as of 2026-09-16). Both cookbooks add a throwaway `uid` field to the state on every call and note that the setup "cannot separate sensitivity to the irrelevant field from variation that would occur on identical requests." Whether identical requests return identical answers is therefore unverified from these sources.

Classifying RAG passages. One request per retrieved passage with four Nouls (relevant, usable evidence, contradicts the premise, tries to instruct the model), routed by thresholds in code, with a planted prompt injection caught in the example (source: https://docs.typesafe.ai/cookbooks/classifying_rag_passages.md, as of 2026-09-16). The relevance for Warrant is the "is this text trying to instruct the model" Noul, which is a cheap check on diff hunks and comments that an agent might use to steer a judgment.

## 5. Evidence and its limits

Methodology of the workflow evals. TypeSafe built four workflows in code (security incidents, agent trace observability, invoice processing, customer service), each a fixed graph of Noul, Choice, and Score questions plus deterministic rules. Every model is run through the same graph at its provider's default reasoning setting. "Instead of debating the correctness of the harness and labels, we assume that the code is correct," and the reference labels are "an average of the responses of GPT-6 Astra and Claude Fable 5.1, both at high thinking" (source: https://evals.typesafe.ai/, as of 2026-09-16). The LLM conditions use TypeSafe's open-source adapter to force the same typed-output shape (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16).

The numbers, from the evals overview, each a mean over the four workflows against consensus labels, with cost and time per case (source: https://evals.typesafe.ai/, as of 2026-09-16):

| Model, workflow condition | Accuracy | Cost per case | Time per case |
| --- | --- | --- | --- |
| GPT-5.6 Sol | 74.1% | $0.0836 | 23.3 s |
| Claude Opus 5 | 73.1% | $0.1761 | 37.8 s |
| GPT-5.6 Terra | 67.9% | $0.0304 | 10.1 s |
| Jev | 67.8% | $0.0004 | 0.4 s |
| Claude Sonnet 5 | 67.8% | $0.1174 | 78.1 s |
| Luna | 66.8% | $0.0033 | 12.9 s |
| DeepSeek v4 pro | 65.5% | $0.0413 | 86.5 s |
| DeepSeek v4 flash | 64.4% | $0.0059 | 51.9 s |
| Claude Haiku 4.5 | 53.6% | $0.0195 | 12.5 s |

Per workflow, Jev scored 61.7 percent on security incidents (Opus 5 66.2, Sol 62.5), 71.6 percent on agent trace observability (Opus 5 75.2, Sol 76.6, Luna 76.1), 61.8 percent on invoice processing (Opus 5 78.4, Sol 79.1), and 76.0 percent on customer service (Opus 5 72.4, Sol 78.3, DeepSeek v4 flash 76.8) (source: https://evals.typesafe.ai/, as of 2026-09-16). The spread matters: Jev is competitive on the two triage-shaped tasks and about 17 points behind the leaders on invoice processing, the task with the most cross-document reconciliation. That reading is my inference from the published per-workflow figures.

Speed and cost claims. The announcement claims 70 to 500 milliseconds end to end and prices input at $0.042 per million tokens with output free. It also states that the published evals "are generally run from our laptops on the West Coast (this is where our service is currently based)," and that the headline "193.6x faster, 444.6x cheaper" figures are "on the higher end of real world gains" (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). On sustainability, the same post says "We can't prove it isn't subsidized," while the homepage FAQ says "We can serve Jev profitably at our current prices" (sources: same post and https://typesafe.ai/, as of 2026-09-16).

The "cannot hallucinate" claim. The announcement's chart puts Jev at zero hallucination and then says in its own nuance note: "Our number is not empirical. Schema matching is guaranteed, thus we can confidently add 0% into the plots," while the LLM numbers "are from OpenRouter" (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). The Decoder's reading is the right one: "that guarantee only covers the allowed output structure ... A factually wrong choice within those options is still possible" (source: https://the-decoder.com/former-openai-researcher-builds-an-ai-model-that-judges-options-instead-of-writing-text/, as of 2026-09-16). The Register says the claim "really isn't a fair comparison as its output is not natural language" (source: https://www.theregister.com/ai-and-ml/2026/09/16/typesafe-ai-debuts-model-for-machines-that-plays-doom/5296711, as of 2026-09-16). TypeSafe's own docs agree: "Typed output guarantees the interface, not truth" (source: https://raw.githubusercontent.com/typesafe-ai/skills/HEAD/skills/typesafe-ai/SKILL.md, as of 2026-09-16).

Their stated biases. The announcement lists them: the workflows "were made by individuals on our model capabilities team, so some bias could exist," the reference labels bias results "towards OpenAI and Anthropic's models," and the side-by-side demo's short state "paints our model in an advantageous light" (source: https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16). The Decoder adds that Astra, one of the two reference models, does not appear as a competitor on the chart (source: https://the-decoder.com/former-openai-researcher-builds-an-ai-model-that-judges-options-instead-of-writing-text/, as of 2026-09-16). The antibenchmaxxing post commits to dated snapshots and no hill-climbing (source: https://typesafe.ai/blog/antibenchmaxxing, as of 2026-09-16), which also means no cross-lab benchmark comparison exists.

Third-party evidence. The only independent hands-on published as of this date is by Mike Taylor, head of evals at Every (2026-09-15, updated 2026-09-17). Jev answered 21 questions about 37 documents, 777 judgments, "in less than 0.7 seconds" for "an estimated quarter of a cent," and 1,709 judgments across 11 experiments for under a cent. In a second test by Every's CEO, four writing checks over 12 synthetic passages ran at a median 0.35 seconds per passage versus 8.83 seconds for Claude Fable 5.1 at high effort, roughly 25 times faster and about 580 times cheaper, and Jev "caught six of the seven intended defects; Fable caught all seven." The missed defect was an unexplained action, and the author's caveat is "I'd want a more thorough accuracy check before putting it into production" (source: https://every.to/also-true-for-humans/mini-vibe-check-typesafe-s-jev-judged-everything-i-ve-written-in-0-7-seconds, as of 2026-09-16). The lab report behind it, generated 2026-08-28 against a pre-launch model id, includes a "Code repository RAG" experiment over 8 synthetic files with recall at 1 of 100 percent, an "Agent action firewall" over 10 scenarios with 100 percent decision agreement, and a "Judge grid" with 89 percent label agreement and 0.0013 repeat variation. Its own limitations section calls the repository "a clean toy repository" and the navigation race "not a statistically powered agent benchmark" (source: https://typesafe-parallel-judgment-lab.every-4573.chatgpt.site/downloads/experiment-report.md, as of 2026-09-16).

No evidence exists on source-code taste tasks. Nothing published by TypeSafe or a third party measures whether Jev can judge duplication of responsibility, module depth, metric gaming, or intent conformance in real diffs. The codebase examples that exist are file retrieval over toy trees. Every claim in section 8 about accuracy on code is therefore a hypothesis to test, not a finding.

## 6. Operational facts

Access. Jev is "available today in early access" behind a waitlist, and the press release says it is "waitlisted at typesafe.ai" (sources: https://typesafe.ai/blog/introducing-system-one-models-and-jev and https://www.businesswire.com/news/home/20260915525333/en/TypeSafe-AI-Emerges-From-Stealth-With-%2440M-in-Funding-With-New-Model-for-Composable-AI, as of 2026-09-16). Once admitted, the console provides a playground and API keys (source: https://docs.typesafe.ai/introduction/quickstart.md, as of 2026-09-16). Several reviewers noted they could not obtain a key (source: https://kingy.ai/blog/typesafe-jev-review-the-ai-model-that-doesnt-generate-text/, as of 2026-09-16). Whether Warrant can get a key on a useful timeline is unverified.

Privacy. The privacy policy (last updated 2025-11-19) states: "We will not train or fine tune any artificial intelligence or machine learning models on your prompts or other Input," and separately that TypeSafe "will not disclose any Input to a third party other than our service providers." The services "are hosted in the United States" (source: https://typesafe.ai/legal/privacy-policy, as of 2026-09-16).

DPA highlights. The data processing addendum (last updated 2026-04-24) casts the customer as controller and TypeSafe as processor, promises no CCPA "sale" or "share" of customer personal data, gives 15 days to object to a new subprocessor, requires security incident notice "within 72 hours," allows one customer audit every 12 months at the customer's cost, and incorporates the EU standard contractual clauses (modules 2 and 3, Irish governing law) and the UK addendum (source: https://typesafe.ai/legal/dpa, as of 2026-09-16).

Data residency and subprocessors. The trust center lists four subprocessors, all in the USA: Amazon Web Services ("Customer information for live requests is stored and processed on databases, caches and compute nodes within AWS"), Modal ("Customer AI prompts are processed, but not stored, on compute nodes managed by Modal"), Slack, and Google Workspace (source: https://trust.typesafe.ai/subprocessors, as of 2026-09-16). No region choice is offered. For Warrant this means any hunk sent to Jev leaves the machine for US infrastructure, which the wish list treats as needing an explicit opt-in mode (source: docs/design/2026-09-16-agent-builder-wishlist.md, W10).

Rate limits and SLAs. The API docs acknowledge 429 and 529 responses but publish no numeric limits (source: https://docs.typesafe.ai/api.md, as of 2026-09-16). The public terms page (last updated 2026-09-14) is a website terms of use with "as is" disclaimers and no service level commitment (source: https://typesafe.ai/legal/terms, as of 2026-09-16). An API service agreement with an SLA, if one exists for early-access customers, is not public.

Pricing page facts. There is no separate pricing page. The homepage states "$42 Per Billion input tokens" and "238x Lower input price than Claude Fable 5.1," and the announcement gives $0.042 per million input tokens with output free (sources: https://typesafe.ai/ and https://typesafe.ai/blog/introducing-system-one-models-and-jev, as of 2026-09-16).

Versioning. `jev-latest` is the documented alias and SDK default. Pinned ids such as `jev-1.12` appear in cookbooks. The response reports the model that actually answered (sources in section 3). TypeSafe's FAQ on determinism: "Determinism means returning the same result for an identical input. This is less valuable than consistency ... Jev is designed for consistency" (source: https://typesafe.ai/, as of 2026-09-16).

The open-source adapter. `system-one-adapter-python` (MIT, version 0.1.4 on 2026-09-16, initial 0.1.3 on 2026-09-15) is "a drop-in replacement" for the SDK's `system_one` call "backed by LLM APIs instead of TypeSafe," with OpenAI and Anthropic providers plus any OpenAI-compatible endpoint. It builds a per-request JSON schema whose top-level keys are the caller's question ids, forces each Choice answer to exactly the allowed labels, supports `llm_answer_mode` of `probabilities` or `discrete`, normalizes malformed distributions, and retries on schema failures with a correction message. Its response is a subclass of the SDK response with attempt-level debug data (sources: https://raw.githubusercontent.com/typesafe-ai/system-one-adapter-python/HEAD/README.md and https://raw.githubusercontent.com/typesafe-ai/system-one-adapter-python/HEAD/src/system_one_adapter/_schema.py, as of 2026-09-16). This matters for vendor neutrality: the request and answer shape is served from three families today, so a Warrant profile written against the shape is not written against Jev. What does not transfer is calibration. An LLM asked to emit probabilities under a schema is not RLCD-trained, and TypeSafe's own consistency cookbooks show LLM probability conditions varying more than Jev's (source: https://docs.typesafe.ai/cookbooks/consistency_noul_cookbook.md, as of 2026-09-16).

The agent skill. The `typesafe-ai` skill (MIT) installs as a Claude Code plugin from the `typesafe-ai/skills` marketplace or through skills.sh. It tells the agent to read the live docs, to batch questions, to keep policy explicit, and it states plainly that "System One models are trained for calibrated decisions; validate their performance in the target domain" (sources: https://docs.typesafe.ai/agent-skill.md and https://raw.githubusercontent.com/typesafe-ai/skills/HEAD/skills/typesafe-ai/SKILL.md, as of 2026-09-16).

## 7. External coverage

- Business Wire press release, 2026-09-15: $40 million seed led by DCVC, founders named, "up to 100 times faster and less expensive," early access waitlisted. Primary source for funding (https://www.businesswire.com/news/home/20260915525333/en/TypeSafe-AI-Emerges-From-Stealth-With-%2440M-in-Funding-With-New-Model-for-Composable-AI, as of 2026-09-16).
- The Register, Thomas Claburn, 2026-09-16: original reporting with the Doom demo as the hook, notes the hallucination framing "isn't a fair comparison," and adds the Jevons paradox context (https://www.theregister.com/ai-and-ml/2026/09/16/typesafe-ai-debuts-model-for-machines-that-plays-doom/5296711, as of 2026-09-16).
- The Decoder, Maximilian Schreiner, 2026-09-16: the most careful critique, on self-built workflows, model-generated reference labels, Astra's absence, and "No hallucinations doesn't mean no mistakes" (https://the-decoder.com/former-openai-researcher-builds-an-ai-model-that-judges-options-instead-of-writing-text/, as of 2026-09-16).
- Every, Mike Taylor, 2026-09-15: the only independent hands-on, with the 12-passage comparison against Fable 5.1 and the "code linter for knowledge work" framing (https://every.to/also-true-for-humans/mini-vibe-check-typesafe-s-jev-judged-everything-i-ve-written-in-0-7-seconds, as of 2026-09-16).
- Tech Startups, Daniel Levi, 2026-09-16: press-release rewrite with an added quote from Almeida's X post (https://techstartups.com/2026/09/16/typesafe-ai-an-ai-startup-founded-by-chatgpt-co-inventor-emerges-from-stealth-with-40m-to-build-ai-thats-100x-faster-and-cheaper/, as of 2026-09-16).
- orcarouter.ai, 2026-09-16: a long synthesis that leans on Every's numbers and lands on "close to a mid-tier model's judgment at a fraction of a cent per call." Secondary, but the tiering of evidence is sound (https://www.orcarouter.ai/blog/jev-typesafe-system-one-what-we-know, as of 2026-09-16).
- Thin rewrites, none with original testing or reporting: kingy.ai, developersdigest.tech, progressiverobot.com, actionbox.cloud, Gadgets Now, NewsBytes, AI Buzz Wire, LavX News, elsolitario.org, top5apps.ai, agenccy.ai, mohammedshehu.com, and llmreference.com (all as of 2026-09-16, URLs in section 10). Several state that they did not obtain API access.

## 8. Fit for Warrant

The vision draws the line this report has to respect: Warrant "is not an AI judge," model assistance "is always labeled as a model's opinion, never the source of a hard rule," and taste lives in a profile that is "a file," where "Some of those are measurable, and Specgate consumes the measurements. Some are judgment, and Specgate labels the judgment as judgment" (source: docs/vision.md (the Warrant copy of the Specgate vision draft)). The wish list adds that optional model judgment "should remain attributable, bounded, and separate from deterministic findings" and that no source upload should be implied by ordinary checks (source: docs/design/2026-09-16-agent-builder-wishlist.md). Each hypothesis below is tested against those constraints and against what sections 3 through 6 established.

Budget arithmetic used throughout, derived from published figures and marked as my inference: the docs equate about 32,000 tokens with about 150,000 characters, so roughly 4.7 characters per token. A 200-line hunk is about 8,000 characters, or about 1,700 tokens. A contract record with intent, owner, and enforcement notes is about 1,000 to 3,000 characters. Map context for the touched symbols (owner module, consumers, the existing interface that would satisfy the design) is about 5,000 to 20,000 characters if Warrant sends signatures and paths rather than bodies. A 30-question profile at 300 characters each is about 9,000 characters. The sum is about 25,000 to 40,000 characters, under a third of the budget. At $0.042 per million input tokens a full 32,000-token call costs about $0.0013, so a thousand hunk evaluations a day is under $1.50. The constraint that binds is not cost, it is that everything the model needs must be in the state, because it cannot open a file.

(a) The taste profile as the primary home. What supports it: the shape is exactly the guardrails cookbook, a battery of Nouls and Scores over one state, thresholds grouped into named policies, precedence in code (section 4). The independence of questions means each profile rule is its own question with no cross-contamination, which is what "attributable" requires. The cost makes per-hunk evaluation on every candidate, on every rerun, affordable. Every's "code linter for knowledge work" framing is this idea stated by someone who ran it (section 5). What undercuts it: no evidence on code taste exists. The one defect Jev missed in Every's test was the reasoning-shaped one, an unexplained action, and the deep-versus-shallow module judgment in W11 is reasoning-shaped. Per-hunk evaluation sees one hunk, so "a second cache because the agent didn't know about the first" is only detectable if the map context in the state names the first cache, which is Warrant's job, not the model's. Consistency is not determinism, so two runs can straddle a threshold, as the consistency cookbooks show (section 4). And a Score cannot measure depth without describable levels, which W11 says must not collapse to a lines ratio. What would settle it: a labeled corpus of 200 to 500 real hunks from Atlas and Specgate history, labeled by Trey against the profile's questions, run through Jev and through the adapter shape on Opus 5, Sonnet 5, and Haiku 4.5, measuring agreement with the labels, a reliability diagram per question (does 0.8 mean 80 percent), and repeat variance over 10 runs. Adopt a rule only where its reliability curve is monotone and its threshold band is stable across repeats.

(b) Intent matching as a Noul. What supports it: the citation-check cookbook is the same problem, does this text support this claim, solved with one typed question after deterministic filtering, with a confidence gate to review. A Noul phrased as "does `hunk` contradict `contract.intent`" is cheap and fits the budget with the contract text in the state. What undercuts it: a contract's intent is usually implemented across several hunks and several files, so "does this hunk implement intent X" is often not answerable from one hunk, and a Noul near 0.5 means ambiguity, not partial implementation (section 3). The better shape is a Choice per hunk, {advances, unrelated, contradicts, cannot tell from this hunk}, with "cannot tell" as an explicit option, plus one aggregate question over the whole candidate's diff when it fits the budget. What would settle it: take the contracts that already carry an intent field, pair each with historical diffs that a human tagged as implementing, unrelated, or violating, and measure whether the contradicts option separates the violating diffs at a usable threshold.

(c) Verifying agent self-reports such as "pure refactor." What supports it: TypeSafe's own agent-trace eval asks "every claim in the final message is backed by the record" as a Noul, and Every's agent action firewall reached 100 percent decision agreement on its 10 scenarios (section 5). Asking "does `hunk` change observable behavior" and "does `hunk` add, remove, or weaken a test assertion" per hunk is cheap. What undercuts it: behavior preservation is a semantic property that reading a diff cannot prove, and Warrant already has stronger evidence for it, the test receipts, type checks, and Fallow findings that W12 says to consume. A Jev answer here is a triage flag, useful precisely when deterministic evidence is missing or the agent's claim is broader than the evidence covers. Ten scenarios is not a sample. What would settle it: mine commits labeled refactor with green test receipts, plant behavior changes in a copy, and measure detection and false-alarm rates. Route positives to the approval facet as "self-report not corroborated," never to compliance.

(d) Bulk census classification of legacy repositories. What supports it: the hierarchical classification cookbook already walks a codebase tree with beam search, Every's toy retrieval scored perfect recall at 1, and the arithmetic is friendly: a 10,000-file repository at about 1,000 tokens per file is about 10 million tokens, about $0.42, parallelizable at sub-second latency (my inference from the published price and the 70 to 500 millisecond claim). What undercuts it: the census's first job, classifying files as source, test, config, generated, or vendored, is deterministic from paths, headers, and the build graph, and should stay deterministic, because the vision says the map is built "from the real compiler's resolution." Where Jev helps is the second job, proposing observed responsibilities and owners, which is a Choice over modules, and modules can exceed 255, which forces the tree walk. Every census proposal is by definition a suggestion for ratification, so this use is safe with respect to (e). What would settle it: run the walk over Atlas and one messy legacy repo, compare proposed owners with the ratified contracts, and measure how many proposals a human accepts unchanged.

(e) The line not to cross. Never in the deterministic compliance lane. Probabilities go in the receipt, the verdict stays categorical, and judgments route to human review through the approval facet, never to fail. Everything in sections 3 through 6 supports this line rather than undercutting it: TypeSafe says calibration "does not guarantee that an individual answer is correct," that "Typed output guarantees the interface, not truth," and that Jev "can choose the wrong one." Consistency without determinism means a receipt cannot be re-verified by rerunning the model, so a receipt must record the answer as an observation: the state digest, the question digest, the model string from the response (not the alias sent), the usage counts, the request id, and the timestamp. A rerun is a new opinion, and Warrant's completeness accounting must treat an unavailable judgment backend as "judgment not obtained," a distinct state from either pass or fail. One design risk: if CI treats "needs review" as blocking, threshold policies become de facto gates. That is acceptable only because the blocking fact is a pending human decision in the approval facet, which the vision already defines, and never a compliance failure. What would settle it: nothing empirical, this is a design rule. Encode it as a conformance test (W30) that fails the build if any judgment answer can reach the compliance facet.

(f) Adopt the shape as Warrant's judgment-lane interface. What supports it: the shape is small, versioned (v1), served from a one-endpoint API, two SDKs, and an adapter that produces the same answers from OpenAI and Anthropic models, so a profile written as questions and thresholds is portable across backends (section 6). The shape forces the discipline Warrant wants anyway: questions and thresholds in one reviewable file, independent atomic judgments, probabilities rather than prose. Implementing the client in Rust is one POST with a JSON body. What undercuts it: the shape carries no explanation, and W19 wants "the smallest meaningful cause." Warrant must compose the finding text from the question, the answer, and the state it sent, or make a second call to a text model for a rationale, clearly labeled. The adapter is Python, so Warrant would write its own Rust adapter against the same schema-per-request idea, or shell out. One state per request means per-hunk calls multiply, so caching keyed by (state digest, question digest, model id) is required, and the model id in the key must be the response's model string. The confidence formula changed once already between preview and v1, so thresholds are bound to a model version and the profile should record which. Calibration does not transfer to LLM backends, so a profile tuned on Jev is not tuned on Sonnet. What would settle it: build the Rust client and adapter behind one trait, run the corpus from (a) through both, and confirm that the profile file needs no edits to switch backend while the thresholds do.

## 9. Open questions and risks

- Access. Early access is waitlisted with no stated timeline (section 6). Warrant cannot depend on a key it does not have, so the judgment lane must ship with the adapter shape working first.
- Pricing sustainability. TypeSafe says it cannot prove the price is unsubsidized and separately says it serves Jev profitably (section 5). Design for the price to change, not for it to hold.
- Accuracy on code. No evidence beyond a toy repository (section 5). Every claim about taste is a hypothesis until the corpus in section 8 (a) exists.
- Vendor lock. The shape is portable through the adapter, calibration is not (section 6). Thresholds are per backend and per model version.
- Latency from CI regions. Published latencies are from the West Coast to US-hosted infrastructure (sections 5 and 6). CI runners elsewhere will see more. Unverified.
- Non-determinism across versions. `jev-latest` can change under a profile, the response model string can differ from the alias, and the confidence computation has changed once (section 3). Pin a model id where the account allows it, record the response model in every receipt, and re-run the reliability check on every model change.
- Rate limits. 429 and 529 exist and no numbers are published (section 6). A census over a large repository could hit them. Unverified.
- Token budget behavior. The 32,000-token figure is "around," no tokenizer is public, and the failure mode on overflow is unverified (section 3). Warrant should chunk conservatively and treat a 422 as "judgment not obtained."
- Data leaving the machine. Hunks and map context go to US-hosted AWS and Modal (section 6). This must be an explicit opt-in mode with the profile declaring which backend it uses, consistent with W10.
- Caching. No server-side caching is documented, and the consistency cookbooks add a `uid` to defeat repeat effects without saying whether identical requests repeat exactly (section 4). Warrant's own cache keyed by (state digest, question digest, model id) is the only cache to rely on, and a cache hit is a stored opinion, not a fresh one.
- Prompt injection through diffs. Comments and strings in a hunk can address the model. The RAG cookbook's "is this text trying to instruct the model" Noul is a cheap first defense and should be a standing question in every profile (section 4).

## 10. Sources

All accessed 2026-09-16.

TypeSafe first-party:
- https://typesafe.ai/blog/introducing-system-one-models-and-jev (announcement, with FAQ answers read from the rendered page)
- https://typesafe.ai/ (homepage claims and FAQ answers read from the rendered page)
- https://typesafe.ai/manifesto
- https://typesafe.ai/team
- https://typesafe.ai/blog/the-bitterest-lesson
- https://typesafe.ai/blog/antibenchmaxxing
- https://typesafe.ai/legal/privacy-policy
- https://typesafe.ai/legal/dpa
- https://typesafe.ai/legal/terms
- https://trust.typesafe.ai/subprocessors
- https://evals.typesafe.ai/ and https://evals.typesafe.ai/agent_trace_observability.html
- https://api.typesafe.ai/openapi.json and a live unauthenticated probe of https://api.typesafe.ai/v1/models
- https://docs.typesafe.ai/llms.txt (index)
- https://docs.typesafe.ai/api.md
- https://docs.typesafe.ai/concepts/system-one.md
- https://docs.typesafe.ai/concepts/state.md
- https://docs.typesafe.ai/concepts/how-to-build-with-system-one.md
- https://docs.typesafe.ai/concepts/use-case-map.md
- https://docs.typesafe.ai/introduction/machine-learning-primer.md
- https://docs.typesafe.ai/introduction/quickstart.md
- https://docs.typesafe.ai/primitives.md
- https://docs.typesafe.ai/primitives/choice.md
- https://docs.typesafe.ai/primitives/score.md
- https://docs.typesafe.ai/primitives/noul.md
- https://docs.typesafe.ai/primitives/advanced.md
- https://docs.typesafe.ai/confidence.md
- https://docs.typesafe.ai/patterns.md
- https://docs.typesafe.ai/patterns/fan-out.md
- https://docs.typesafe.ai/patterns/confidence-routing.md
- https://docs.typesafe.ai/patterns/composite-scoring.md
- https://docs.typesafe.ai/patterns/intent-routing.md
- https://docs.typesafe.ai/cookbooks/llm_guardrails.md
- https://docs.typesafe.ai/cookbooks/citation_check.md
- https://docs.typesafe.ai/cookbooks/hierarchical_classification.md
- https://docs.typesafe.ai/cookbooks/sde_cascade.md
- https://docs.typesafe.ai/cookbooks/consistency_noul_cookbook.md
- https://docs.typesafe.ai/cookbooks/consistency_choice_cookbook.md
- https://docs.typesafe.ai/cookbooks/parallel_questions.md
- https://docs.typesafe.ai/cookbooks/classifying_rag_passages.md
- https://docs.typesafe.ai/cookbooks/pre_parsed_value_extraction_cookbook.md
- https://docs.typesafe.ai/migrating-to-v1.md
- https://docs.typesafe.ai/agent-skill.md
- https://docs.typesafe.ai/sdk/python/changelog.md
- https://docs.typesafe.ai/sdk/python/api/retries.md
- https://docs.typesafe.ai/sdk/javascript/changelog.md
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-js/HEAD/src/types.ts
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-js/HEAD/src/version.ts
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_core/question_types.py
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_core/response_types.py
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_core/errors.py
- https://raw.githubusercontent.com/typesafe-ai/typesafe-sdk-python/HEAD/src/typesafe_sdk/_schemas/models.py
- https://raw.githubusercontent.com/typesafe-ai/system-one-adapter-python/HEAD/README.md
- https://raw.githubusercontent.com/typesafe-ai/system-one-adapter-python/HEAD/src/system_one_adapter/_schema.py
- https://raw.githubusercontent.com/typesafe-ai/system-one-adapter-python/HEAD/docs/changelog.md
- https://raw.githubusercontent.com/typesafe-ai/skills/HEAD/skills/typesafe-ai/SKILL.md
- https://github.com/typesafe-ai (organization listing via the GitHub API)

Press and third parties:
- https://www.businesswire.com/news/home/20260915525333/en/TypeSafe-AI-Emerges-From-Stealth-With-%2440M-in-Funding-With-New-Model-for-Composable-AI
- https://www.theregister.com/ai-and-ml/2026/09/16/typesafe-ai-debuts-model-for-machines-that-plays-doom/5296711
- https://the-decoder.com/former-openai-researcher-builds-an-ai-model-that-judges-options-instead-of-writing-text/
- https://every.to/also-true-for-humans/mini-vibe-check-typesafe-s-jev-judged-everything-i-ve-written-in-0-7-seconds
- https://typesafe-parallel-judgment-lab.every-4573.chatgpt.site/ and https://typesafe-parallel-judgment-lab.every-4573.chatgpt.site/downloads/experiment-report.md
- https://techstartups.com/2026/09/16/typesafe-ai-an-ai-startup-founded-by-chatgpt-co-inventor-emerges-from-stealth-with-40m-to-build-ai-thats-100x-faster-and-cheaper/
- https://www.orcarouter.ai/blog/jev-typesafe-system-one-what-we-know
- https://kingy.ai/blog/typesafe-jev-review-the-ai-model-that-doesnt-generate-text/
- https://www.developersdigest.tech/blog/typesafe-jev-system-one-models-release-guide-2026
- https://www.progressiverobot.com/2026/09/16/jev-model-typesafe-programmatic-logic/
- https://actionbox.cloud/blog/typesafe-ai-jev-review/
- https://gadgetsnow.indiatimes.com/featured/typesafe-jev-answers-software-in-70ms-for-42-a-billion-tokens/articleshow/134284633.cms
- https://www.newsbytesapp.com/news/science/chatgpt-co-inventor-unveils-jev-a-new-kind-of-frontier-ai/story
- https://aibuzzwire.news/en/article/typesafe-ai-jev-system-one-model-structured-decisions-ex-openai
- https://news.lavx.hu/article/typesafe-ai-launches-system-one-models-claims-100x-speed-gains-over-frontier-llms
- https://elsolitario.org/en/2026/09/16/typesafe-ai-jev-structured-decision-model/
- https://top5apps.ai/news/typesafe-jev-is-a-model-that-doesnt-talk-the-future/
- https://agenccy.ai/news/jevs-0-percent-hallucination-sits-beside-a-678-percent-accuracy-score/
- https://mohammedshehu.com/jev-typesafe-ai/
- https://www.llmreference.com/model/jev

Warrant and Specgate internal:
- docs/vision.md (the Warrant copy of the Specgate vision draft)
- docs/design/2026-09-16-agent-builder-wishlist.md
