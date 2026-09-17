# AI slop is a contract problem

Status: vision draft, September 16, 2026. Author: Trey Goff, drafted with Fable. This is the announcement post for the rebuilt Specgate, now called Warrant, written before the product exists and describing it as it will be. It is a vision, not a spec or a plan. Statements about Atlas are the intended outcome, not observed results.

*Warrant: architecture your agents can read and can't cheat.*

The feature shipped, the tests passed, and the codebase got worse.

If you run coding agents in a real repository you know this Tuesday. Somewhere in the diff there is a second cache, because the agent didn't know about the first one. A third validator. A helper nobody calls. A function named `authorize` that the request path never touches. You didn't ask for any of it, the reviewer didn't catch it, and every individual change looked reasonable on its own. Do that forty times and you have a codebase that works and that nobody, human or machine, can safely change.

I run more coding agents than almost anyone. For a year I have been losing to this, and I have tried every fix on the menu: longer instruction files, stricter linters, review agents reviewing the implementing agents, cleanup passes that find the slop after the fact and scrape it off. All of it helps. None of it holds, because all of it is downstream. The slop is generated faster than it can be reviewed, and a cleanup pass on Friday doesn't change what the agent does on Monday.

## Why agents produce slop

Agents don't write slop because they're bad at code. They write it because they're guessing.

Where does this behavior belong? What already owns it? Which interface am I supposed to use? What must I not break? What proof will be required before this is accepted? A senior engineer answers those from memory, because the architecture lives in her head. An agent has no head. It has the files it happened to open, a chat transcript, and a CLAUDE.md that says "keep it clean." So it guesses, and the residue of guessing is a second cache.

Then it gets worse. When a check does fail, the agent holds the pen for the rules too. The baseline file, the ignore comment, the lint config, the "approved by the lead" note in a YAML file: all of it is text in the working tree, and the agent can edit text. A gate the implementer can edit is a suggestion.

For fifty years the architecture of a program lived in people's heads and degraded every time one of them left. Agents made the degradation instant. They also made the fix possible, because for the first time the thing reading the architecture is a machine, and a machine can read a contract.

## What Warrant does

Warrant makes architectural intent executable. You declare the architecture once, in your own vocabulary. Agents get it as context before they edit. A gate checks the actual candidate against it afterward and produces a verdict that cannot collapse "we didn't look" into "it passed." And the rules are signed by a human key the agents don't have.

Three moves, in order. The map before the edit. The contract during. The receipt after.

## A change, start to finish

Say an agent is asked to add grouped undo to a meeting debrief in Atlas, the product my agents are building.

It asks Warrant for the task's architectural context and gets back facts, with file paths and symbols: the module that owns debrief commands, the transaction interface it must use, the existing compensation mechanism for undo, the public entrypoints that expose it on three transports, the tests that already cover the flow. Facts are marked as facts. Suggestions about the future design are marked as suggestions.

It describes the change it intends to make. Warrant answers with the consumers that will be affected, the files another lane is already touching, and one warning: the proposal introduces a second undo dispatcher, and `actions/undo` already owns that responsibility with four consumers. Reuse it or explain why the responsibilities differ.

It edits. Feedback follows the edited operation through imports and registrations rather than checking only the touched file. When something violates a contract, the finding names the contract, shows the shortest import or registration path that proves it, and points at the existing interface that would satisfy the design.

It runs the focused checks. Warrant says what those checks establish and what they don't: the race test against a real database is still owed, and one consumer's integration suite hasn't run.

Meanwhile another agent changes a related interface. The first agent's passing result is now stale. Warrant names the invalidated evidence and the affected work. It does not silently reuse a green receipt.

The integrated candidate is submitted. Contracts, required external test receipts, valid exceptions, and pending human decisions are evaluated together against that exact snapshot. The verdict has four facts in it, not one: compliance failed on one contract, analysis was complete, evidence is missing one named test, and one policy change needs approval because it widens what an adapter may send.

I get a paragraph. What changed, which obligations were met, what's uncertain, and that one rule got broader. I read the ruling in plain English, and I sign it with a key that lives on my machine and nowhere the agents can reach. Or I don't, and the gate stays closed.

## The four ideas underneath

Architecture is declared in the domain's vocabulary. A contract says that a module owns approval decisions, that another owns transaction execution, that one adapter is the only permitted route for sending email, that a capability called "undoable operation" requires an entrypoint, an authorization check, a persisted effect, a readback, a failure path, and a test. Each contract carries its intent, its owner, the ruling that established it, how it is enforced, and what that enforcement cannot see. A rule that only recognizes a function named `validate` says so. Filename globs are how you scope a contract, never how you express one.

The map is the product. Before any verdict, Warrant builds an inventory of every file in the snapshot with a classification and a reason: source, test, config, generated and by what, vendored and from where. It builds the program model from the real compiler's resolution, so what it believes about imports and symbols is what the build believes. Agents query it directly. A file that couldn't be read or a construct that couldn't be analyzed appears as missing evidence, not as silence. The denominator is always visible.

Verdicts cannot collapse. Compliance, completeness, evidence, and approval are four separate facts, and a run can carry a verified violation and an incomplete analysis at the same time. Exit zero on the integration gate means every obligation was met by real evidence or a valid signed exception. A missing tool, a parse failure, a truncated report, or an absent test receipt is a failure, never a warning. Every verdict comes with a receipt binding the snapshot digest, the policy digest, the inventory, every tool version, every external evidence digest, and the evaluation clock. A machine can verify a receipt. No model is asked whether it looks legitimate.

Agents propose. Humans sign. An agent may draft a policy change or a ruling; it cannot ratify one. Rulings are canonical records with a scope, a policy digest, a candidate range, and an expiry, signed with an SSH key held by a person. Any edit breaks the signature; a change is a new record that supersedes the old one, signed again. And because agents will relax rules without meaning to, Warrant classifies every policy change as narrowing, widening, or restructuring, and treats "we couldn't tell" as widening. A wider permission is never the fix for a failing check.

## What it's built from

Almost none of this required new machinery, which is the part I find most satisfying. Compilers already know the symbol graph; the TypeScript compiler's own resolution and references feed the program model, and oxc, the Rust toolchain for JavaScript, parses and resolves imports natively so the fast path needs no Node process. Git already is a content-addressed snapshot store; a staged tree, a commit, and an integrated candidate from two lanes are all just tree hashes. ast-grep already matches structural patterns, so a pattern-enforced contract is an ast-grep rule with Warrant metadata on top. Fallow, knip, tsc, and every linter that speaks SARIF supply evidence about dead code, duplication, complexity, and types; Warrant relates that evidence to obligations instead of reinventing the detectors. SSH signatures, the same mechanism git uses for signed commits, sign rulings. SQLite holds the model, one file per snapshot, queryable with SQL that every agent already speaks.

What we wrote: the contract vocabulary and its effective-policy semantics, the four-fact verdict and the receipt that binds it, the policy-widening classifier, and the completeness accounting. Everything else we borrowed from people who did it better.

## What you get without asking

The code map. Engineers have wanted an always-current architecture diagram for thirty years and never had one, because the diagram lived in a wiki and the code lived in git. The model Warrant builds for agents renders as a diagram for humans, and it can't drift, because it is derived from the snapshot every time.

Onboarding without canonizing the mess. `warrant census` reads an existing repository and proposes contracts: observed modules, observed owners, observed dependency directions, observed entrypoints. You ratify the ones that describe the architecture you intend and reject the ones that merely describe the accident you have. Nobody hand-writes YAML from a blank page.

Instruments that can't drift quietly. Every analyzer version is pinned and recorded, and an upgrade produces a before-and-after report on a frozen corpus before it is adopted. A tool getting more permissive shows up as an instrument change, never as an improvement in the code.

Many agents, one truth. Each lane's evidence is valid for that lane's snapshot. The integrated candidate is a new snapshot, evaluated fresh, with new findings attributed to the combination and not to whichever file happened to merge last.

## What it refuses to be

It is not a linter, a formatter, a test runner, a workflow engine, or a place to manage human accounts. Those exist and they are good. It is not an AI judge: model assistance can propose contracts, explain a finding, or flag a question for review, and its output is always labeled as a model's opinion, never the source of a hard rule. And it is not a score. There is no "slop: 87." There are contracts, evidence, and unresolved obligations, in a form a person can read and a machine can check.

## Taste is a profile

My rules are mine. Clean Code as scripture, root causes over band-aids, no god files, no god functions, modules that hide something real behind an interface a caller can understand without reading the implementation. Some of those are measurable, and Warrant consumes the measurements. Some are judgment, and Warrant labels the judgment as judgment. Either way they live in a profile, and the profile is a file. You can disagree with every one of my preferences, write your own, and the contract machinery underneath doesn't care.

## The missing piece

The software factory was never missing more agents. It was missing a contract the agents can read and can't cheat.

Agents with the map stop guessing. Agents that can't sign the rules stop relaxing them. Verdicts that can't collapse stop laundering incomplete analysis into green. And the human at the top gets less supervision without less control, which is the whole game.

We are building Atlas with it, and the codebase is getting simpler as it grows, which I have never seen happen before.
