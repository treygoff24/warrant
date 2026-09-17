# Signing and attestation for Warrant receipts and rulings

Status: research report, 2026-09-16. Author: Fable (research lane), for Warrant. Sources fetched live on 2026-09-16.

## Scope and method

This report covers the mechanisms Warrant should borrow for the receipt that binds a verdict to its inputs, the ruling by which a human ratifies a policy change or grants an exception, and the verification of both on a machine that trusts neither the agent nor the working tree. It follows the vision ("Agents propose. Humans sign.") and wishes W14, W15, W16, and W24.

Every claim about a tool or spec cites the primary source it came from, fetched on 2026-09-16. Where a behavior was checked by running it, the text says "observed on this machine" with the versions used: OpenSSH_10.0p2 Debian-7+deb13u4, git 2.47.3, gpg 2.4.7. Throwaway keys were generated in a scratch directory for those checks. Where a flag or behavior could not be confirmed against a primary source, the text says "unverified".

The recommendation in one paragraph: sign JCS-canonical bytes with OpenSSH's sshsig format under Warrant-specific namespaces, one namespace per ruling kind; verify with an allowed-signers file that lives outside the candidate; shape receipts and rulings as in-toto Statements with Warrant predicate types; sign gate receipts with a gate key; and keep DSSE and Sigstore as a documented path for teams rather than a v1 requirement.

## 1. SSH signatures: sshsig and the ssh-keygen -Y operations

### 1.1 The format

OpenSSH's PROTOCOL.sshsig describes "a lightweight SSH Signature format that is compatible with SSH keys and wire formats"; "only detached and armored signatures are supported." The armored form is the line "-----BEGIN SSH SIGNATURE-----", a base64 blob wrapped at 76 characters, and "-----END SSH SIGNATURE-----". The blob holds a six-byte "SSHSIG" preamble, a uint32 version (1), the signer's public key, the namespace, a reserved string, the hash algorithm, and the signature. Verifiers "MUST reject signatures with versions greater than those they support." (Source: https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig, revision 1.4 of 2020-08-31, as of 2026-09-16.)

Two details shape Warrant's design. The bytes passed to the signature primitive are not the file: they are the preamble, namespace, reserved string, hash algorithm name, and the hash (sha256 or sha512) of the message, so a slow or remote key signs a fixed-size input. And the namespace is inside the signed data, "MUST NOT be the empty string", and exists to prevent "cross-protocol attacks caused by signatures intended for one intended domain being accepted in another." RSA signatures must use rsa-sha2-256 or rsa-sha2-512, never SHA-1 ssh-rsa. (Source: PROTOCOL.sshsig, as of 2026-09-16.)

Decoding a signature produced on this machine confirmed the layout: version 1, the namespace given on the command line, hash algorithm sha512 (the default), an empty reserved field, and the ssh-ed25519 public key embedded in the blob. Because the key travels inside the signature, a verifier can look it up in a trust list before checking anything else; that is what find-principals does. (Observed, OpenSSH 10.0p2, 2026-09-16.)

### 1.2 The operations, with exact command lines

The ssh-keygen(1) synopsis lists five -Y operations as of OpenSSH 10.0:

```
ssh-keygen -Y find-principals [-O option] -s signature_file -f allowed_signers_file
ssh-keygen -Y match-principals -I signer_identity -f allowed_signers_file
ssh-keygen -Y check-novalidate [-O option] -n namespace -s signature_file
ssh-keygen -Y sign [-O option] -f key_file -n namespace file ...
ssh-keygen -Y verify [-O option] -f allowed_signers_file -I signer_identity -n namespace -s signature_file [-r krl_file]
```

(Source: ssh-keygen(1), OpenSSH 10.0p2 manual page on this machine, and https://man.openbsd.org/ssh-keygen.1, as of 2026-09-16.)

Signing. "If no files are specified then ssh-keygen will sign data presented on standard input. Signatures are written to the path of the input file with '.sig' appended, or to standard output if the message to be signed was read from standard input." The -f key "may refer to either a private key, or a public key with the private half available via ssh-agent(1)", and the namespace "must be provided via the -n flag." (Source: ssh-keygen(1), as of 2026-09-16.)

```
ssh-keygen -Y sign -f ~/.ssh/id_ed25519 -n warrant-ruling-policy@example.com warrant/rulings/r-0042.json
```

Observed: signing a file whose .sig already exists prompts "Overwrite (y/n)?" and blocks; signing from standard input writes the armored signature to stdout with "Signing data on standard input" on stderr. A tool that shells out to ssh-keygen should use the stdin form. (Observed, OpenSSH 10.0p2, 2026-09-16.)

Verifying. ssh-keygen "accepts a message on standard input and a signature namespace using -n", the signature via -s, "the identity of the signer using -I and a list of allowed signers via the -f flag"; a revocation file, "a KRL or a one-per-line list of public keys", goes with -r; success is "a zero exit status." (Source: ssh-keygen(1), as of 2026-09-16.)

```
ssh-keygen -Y verify -f /etc/warrant/allowed_signers -I trey@example.com \
  -n warrant-ruling-policy@example.com -s warrant/rulings/r-0042.json.sig \
  -r /etc/warrant/revoked_keys < warrant/rulings/r-0042.json
```

Observed: success prints Good "namespace" signature for principal with ED25519 key SHA256:... and exits 0; every failure exited 255. The stderr text distinguishes causes: "namespace does not match"; "incorrect signature" for altered bytes; "key is not yet valid" and "key has expired", naming the file and line; "key is not permitted for use in signature namespace"; "Unable to open allowed keys file"; and a bare "Could not verify signature." when the -I identity matches no line or the key is revoked. A caller that wants to explain a failure must capture stderr. (Observed, OpenSSH 10.0p2, 2026-09-16.)

Principals. find-principals prints the principals whose key made the signature (git's error text says it is "available in openssh version 8.2p1+"); match-principals, added in 8.9, reports whether an identity appears in the file. An unknown key produced "No principal matched." and exit 255. (Sources: ssh-keygen(1); https://github.com/git/git/blob/master/gpg-interface.c; https://www.openssh.com/releasenotes.html; observed, 2026-09-16.)

```
ssh-keygen -Y find-principals -s warrant/rulings/r-0042.json.sig -f /etc/warrant/allowed_signers
ssh-keygen -Y match-principals -I trey@example.com -f /etc/warrant/allowed_signers
```

Structure only. check-novalidate "Checks that a signature ... has a valid structure. This does not validate if a signature comes from an authorized signer." Observed: a signature from a key in no trust list passed with exit 0 and a "Good ... signature" line. It is useful for rendering who signed what; it must never be the gate's verdict. (Source: ssh-keygen(1); observed, 2026-09-16.)

Options. With -Y, -O accepts hashalg=sha256|sha512 (default sha512), print-pubkey ("Print the full public key to standard output after signature verification"), and verify-time=timestamp, "a time to use when validating signatures instead of the current time" in YYYYMMDD[Z] or YYYYMMDDHHMM[SS][Z] form, local time unless suffixed with Z. Observed: print-pubkey works with verify and is rejected by find-principals as "Invalid option". (Source: ssh-keygen(1); observed, 2026-09-16.)

### 1.3 The allowed-signers file

The file "uses a format patterned after the AUTHORIZED_KEYS FILE FORMAT": per line, "principals, options, keytype, base64-encoded key"; # lines are comments. Principals are "a pattern-list ... of one or more comma-separated USER@DOMAIN identity patterns", and the -I identity "must match a principals pattern in order for the corresponding key to be considered acceptable." The options, case-insensitive, are cert-authority, namespaces="namespace-list" (a pattern list the signature's namespace must match), valid-after="timestamp", and valid-before="timestamp". With certificates, the principal "must match both the principals pattern in the allowed signers file and the principals embedded in the certificate itself." (Source: ssh-keygen(1), ALLOWED SIGNERS, as of 2026-09-16.)

A Warrant trust line, as tested on this machine:

```
trey@example.com namespaces="warrant-ruling-*@example.com,warrant-receipt@example.com",valid-after="20260101Z",valid-before="20271231Z" ssh-ed25519 AAAAC3...
```

(Observed working with sign and verify, OpenSSH 10.0p2, 2026-09-16.)

### 1.4 Why a Warrant-specific namespace matters

ssh-keygen recommends "names following a NAMESPACE@YOUR.DOMAIN pattern to generate unambiguous namespaces." Git signs and verifies commits under the fixed namespace "git" (the source passes "-n", "git" in both directions). If Warrant verified rulings under "git", every commit the human ever signed would be a valid signature over some bytes in Warrant's domain, and every ruling would be a valid commit signature over the ruling bytes. Observed: a trust line restricted to Warrant namespaces was rejected by git verify-commit with "key is not permitted for use in signature namespace "git"", and a commit signature failed under a Warrant namespace with "namespace does not match". One key can be listed twice, once for git and once for Warrant. (Sources: ssh-keygen(1); https://github.com/git/git/blob/master/gpg-interface.c; observed, 2026-09-16.)

### 1.5 Key types

ssh-keygen generates ecdsa, ecdsa-sk, ed25519, ed25519-sk, and rsa keys. FIDO keys have "a per-device private key that is unique to each FIDO authenticator and that cannot be exported", and "FIDO authenticators generally require the user to explicitly authorise operations by touching or tapping them." Certificates are keys plus identity, principals, and options "signed by a Certification Authority (CA) key", produced with ssh-keygen -s /path/to/ca_key -I key_id /path/to/user_key.pub and accepted by a cert-authority trust line. An ed25519-sk key gives a physical confirmation per ruling with no Warrant code; an SSH CA lets a team add signers without editing the file. Whether GitHub verifies commits signed with -sk keys or certificates is not stated on the GitHub pages read for this report (https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification, as of 2026-09-16); unverified. (Source: ssh-keygen(1), FIDO AUTHENTICATOR and CERTIFICATES, as of 2026-09-16.)

### 1.6 Passphrase and ssh-agent for the signer

A private key encrypted with a passphrase prompts on every -Y sign that names the private key; naming the public key with the private half in ssh-agent avoids the prompt. ssh-add -c makes an identity "subject to confirmation before being used" through ssh-askpass, and -t sets "a maximum lifetime". The agent's sockets "should only be readable by the owner", and ssh-agent(1) warns that "use of other tools to forward access to the agent socket may circumvent" its restrictions. Observed: after ssh-add, ssh-keygen -Y sign -f signer.pub signed through the agent and verified. For Trey's layout the operational rule is that the trey agent socket must never be reachable by trey-agent, by permissions, forwarding, or a shared TMPDIR; a confirmation-required identity is the software equivalent of a touch. (Sources: ssh-keygen(1), ssh-add(1), ssh-agent(1), OpenSSH 10.0p2; observed, 2026-09-16.)

### 1.7 How git uses sshsig

git-config documents gpg.format ("Default is 'openpgp'. Other possible values are 'x509', 'ssh'"), user.signingKey (a private key path, a public key path when ssh-agent holds the key, or a literal key:: value), gpg.ssh.defaultKeyCommand, gpg.ssh.allowedSignersFile, and gpg.ssh.revocationFile ("Either a SSH KRL or a list of revoked public keys ... always be treated as having trust level 'never'"). Its trust rule is the one Warrant should copy: "the trust level of a signature verification is set to fully when the public key is present in the allowedSignersFile. Otherwise the trust level is undefined and git verify-commit/tag will fail." It also notes that "Since OpensSSH 8.8 this file allows specifying a key lifetime using valid-after & valid-before options" and that "Git will mark signatures as valid if the signing key was valid at the time of the signature's creation." (Source: git-config(1), git 2.47.3, and https://git-scm.com/docs/git-config, as of 2026-09-16.)

The signature sits in the commit's gpgsig header (continuation lines prefixed with a space) or is appended to a tag body, with the same SSH armor lines. The source shows the verification sequence: find-principals against the allowed-signers file; check-novalidate when no principal matched, to display the signature anyway; otherwise -Y verify -n git -f <allowed> -I <principal>, plus -r <revocationFile> when configured, plus -Overify-time= set to the commit's payload timestamp. (Sources: https://git-scm.com/docs/gitformat-signature; https://github.com/git/git/blob/master/gpg-interface.c, as of 2026-09-16.)

Observed with git 2.47.3: git verify-commit exited 0 with Good "git" signature for trey@example.com when the key was listed and 1 otherwise; git log --format='%G? %GS %GK' printed G (good, trusted), U (valid signature, no principal), and B (revoked key, or a line restricted to other namespaces). With valid-before in the past, git reported "key has expired: verify time 2026-09-16T21:22:25 ...", the commit's own timestamp, so git checks key validity at a time the committer chooses. (Observed, 2026-09-16.)

### 1.8 Revocation

KRLs are "binary files [that] specify keys or certificates to be revoked"; ssh-keygen -k -f krl_file keys... creates one, -u updates it, and -Q queries it, exiting nonzero if any key is revoked. -Y verify -r takes a KRL or a plain key list. Observed: both forms blocked an otherwise good signature, and -Q printed "REVOKED" with exit 1. (Source: ssh-keygen(1), KEY REVOCATION LISTS; observed, 2026-09-16.)

### 1.9 Version timeline

From the OpenSSH release notes: 8.1 (2019-10-09) added "an experimental lightweight signature and verification ability" whose signatures "embed a namespace that prevents confusion and attacks between different usage domains"; 8.2 added find-principals; 8.7 added -Oprint-pubkey; 8.9 added match-principals and "selection of hash at sshsig signing time"; 9.1 let "sshsig verification times and authorized_keys expiry-time options ... accept dates in the UTC time zone"; 10.0 made RSA signing pick its algorithm from -Ohashalg. git-config dates valid-after and valid-before to 8.8. A floor of OpenSSH 9.1 covers everything this report relies on, including the Z suffix that makes verify-time unambiguous. (Sources: https://www.openssh.com/releasenotes.html; git-config(1), as of 2026-09-16.)

## 2. Canonicalization: what to sign

### 2.1 RFC 8785, the JSON Canonicalization Scheme

RFC 8785 (JCS) "defines how to create a canonical representation of JSON data by building on the strict serialization methods for JSON primitives defined by ECMAScript, constraining JSON data to the I-JSON subset, and by using deterministic property sorting." Its advantage over the JWS approach of base64-wrapping the payload is that "data can be kept in its original form", so a signed JSON object is still a JSON object. The rules that bite in practice: (Source: https://www.rfc-editor.org/rfc/rfc8785, as of 2026-09-16.)

- Input. Objects "MUST NOT exhibit duplicate property names"; numbers "MUST be expressible as IEEE 754 double-precision values", and for more precision "it is RECOMMENDED to represent such numbers as JSON strings" (Section 3.1, Appendix D).
- Strings. ECMAScript escaping: U+0000 to U+001F as lowercase \uhhhh except \b \t \n \f \r; backslash and double quote escaped; everything else emitted as is. Lone surrogates "MUST cause a compliant JCS implementation to terminate with an appropriate error." No Unicode normalization: components "MUST preserve Unicode string data 'as is'" (Sections 3.1, 3.2.2.2).
- Numbers. ECMAScript Number-to-string serialization; "occurrences of NaN or Infinity MUST cause a compliant JCS implementation to terminate with an appropriate error" (Section 3.2.2.3).
- Ordering. Properties sorted recursively, arrays untouched, comparing names "as arrays of UTF-16 code units" as unsigned integers, not UTF-8 bytes and not locale collation (Section 3.2.3). The RFC's seven-key test object has the expected order "Carriage Return", "One", "Control", "Latin Small Letter O With Diaeresis", "Euro Sign", "Emoji: Grinning Face", "Hebrew Letter Dalet With Dagesh".
- Output. UTF-8, no whitespace (Sections 3.2.1, 3.2.4).

### 2.2 Floats, Unicode, ordering: what Warrant should do

Keep floats out of signed records. Every quantity a ruling or receipt needs is a digest (hex string), a count (an integer far below 2^53), a timestamp (RFC 3339 string), a version, or a path. A fractional metric, if one ever appears, is a decimal string, which Appendix D recommends for money-like values anyway. Then the number-serialization corner of JCS never executes and a serializer that rejects any non-integer number in a signed record is a one-line check.

Treat paths and principal names as opaque code-point sequences. JCS preserves code points and forbids normalization, so a decomposed character stays decomposed; Warrant must never normalize on either side and must reject lone surrogates and invalid UTF-8 at the boundary with an error rather than a substitution character.

Sort by UTF-16 code units. Warrant's own keys are ASCII, so a byte sort would give the same result today; the UTF-16 comparison costs a few lines and keeps the records verifiable by any conforming JCS library later.

### 2.3 Alternatives, and the combination to use

Sign the raw bytes and store them immutably. This is what sshsig does natively; if the file is never rewritten after signing, canonicalization is unnecessary for verification. The failure mode is any tool that reformats or re-serializes the file (an editor, a formatter, a JSON library that reorders keys) and silently invalidates the signature. Canonical form makes the stored bytes the unique serialization, so a round trip through a JCS-aware serializer is harmless and any other round trip is detectable as a byte change.

Detached signatures. sshsig is detached by construction ("only detached and armored signatures are supported"), so the record stays valid JSON with no signature field inside it and the Appendix F dance of stripping a signature property before canonicalizing is avoided. (Sources: PROTOCOL.sshsig; RFC 8785 Appendix F, as of 2026-09-16.)

Sign a digest. Signing a small document that carries the digest of a larger artifact is the in-toto pattern, right for binding a receipt to a snapshot and to evidence files, wrong for the ruling text itself, because the human should sign the bytes they read, not a hash of them.

Recommended combination: JCS-canonical JSON written once and never modified; a detached sshsig over exactly those bytes; a verifier that compares bytes, never structures; and subject fields that carry digests of everything the record refers to. The DSSE protocol's rule applies: "implementations MUST NOT re-parse the envelope after verification to pull out the payload." (Source: https://github.com/secure-systems-lab/dsse/blob/master/protocol.md, as of 2026-09-16.)

## 3. Attestation formats

### 3.1 in-toto Statement v1

The Statement "binds [the attestation] to a particular subject and unambiguously identifying the types of the Predicate." Four fields: _type, always "https://in-toto.io/Statement/v1"; subject, an array of ResourceDescriptors where "Each element MUST have digest set" and "Subjects are assumed to be immutable"; predicateType, a URI; and predicate, an object. Subjects "are matched purely by digest, regardless of content type." A ResourceDescriptor "MUST specify one of uri, digest or content at a minimum" and may carry name, downloadLocation, mediaType, and annotations. (Sources: https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md; https://github.com/in-toto/attestation/blob/main/spec/v1/resource_descriptor.md, as of 2026-09-16.)

The DigestSet vocabulary already names git objects: gitCommit, gitTree, gitBlob, and gitTag are "The lowercase hex SHA-1 (40 character) or SHA-256 (64 character) of a git commit, tree, blob, or tag object", computed over "<type> SP <size> NUL <content>", and "The gitTree and gitBlob in particular can be used for arbitrary trees or files, even outside git." The guidance is "at least sha256 for compatibility ... unless a different hash algorithm is more conventional (e.g. gitCommit for git)." The parsing rules Warrant should adopt for its own predicates: "Consumers MUST ignore unrecognized fields unless otherwise noted", major version in the TypeURI, extension fields allowed. (Sources: https://github.com/in-toto/attestation/blob/main/spec/v1/digest_set.md; https://github.com/in-toto/attestation/blob/main/spec/v1/README.md, as of 2026-09-16.)

Two published predicates are worth reading before inventing anything. Test Result (https://in-toto.io/attestation/test-result/v0.1) has a required result of PASSED, WARNED, or FAILED, a required configuration list, optional passedTests, warnedTests, and failedTests, and states "The expected subject are the source artifacts tested." SLSA's Verification Summary Attestation (https://slsa.dev/verification_summary/v1) is verdict-shaped: verifier.id, timeVerified ("Timestamp indicating what time the verification occurred"), policy as a ResourceDescriptor that "Describes the policy that the subject was verified against" with digest recommended, inputAttestations ("The collection of attestations that were used to perform verification"), and verificationResult, "Either 'PASSED' or 'FAILED'". The VSA is the closest analog to a Warrant receipt, and it falls short exactly where the vision predicts: one binary result where Warrant needs four facets. (Sources: https://github.com/in-toto/attestation/blob/main/spec/predicates/test-result.md; https://slsa.dev/spec/v1.1/verification_summary, as of 2026-09-16.)

### 3.2 DSSE

DSSE defines SIGNATURE = Sign(PAE(UTF8(PAYLOAD_TYPE), SERIALIZED_BODY)) with PAE(type, body) = "DSSEv1" + SP + LEN(type) + SP + type + SP + LEN(body) + SP + body, LEN being the ASCII decimal byte length. Sign() "is an arbitrary digital signature format"; KEYID is "an unauthenticated hint" that "MUST NOT be used for security decisions". The envelope is {"payload": base64, "payloadType": string, "signatures": [{"keyid", "sig": base64}]}, with a (t, n) threshold rule for multiple signatures. The in-toto envelope layer requires payloadType "application/vnd.in-toto+json" or "application/vnd.in-toto.<predicate>+json" and lists what an alternative envelope must have, including "SHOULD avoid depending on canonicalization for security" and "SHOULD NOT require the verifier to parse the payload before verifying". (Sources: https://github.com/secure-systems-lab/dsse/blob/master/protocol.md; https://github.com/secure-systems-lab/dsse/blob/master/envelope.md, version 1.0.2; https://github.com/in-toto/attestation/blob/main/spec/v1/envelope.md, as of 2026-09-16.)

### 3.3 SLSA provenance v1 vocabulary

Provenance v1 (predicateType "https://slsa.dev/provenance/v1") has buildDefinition and runDetails. buildType "Identifies the template for how to perform the build and interpret the parameters and dependencies"; externalParameters are "untrusted; they MUST be included in the provenance and MUST be verified downstream" while internalParameters are "trusted because the platform is trusted"; resolvedDependencies is an "Unordered collection of artifacts needed at build time"; builder.id is a "URI indicating the transitive closure of the trusted build platform"; metadata holds invocationId, startedOn, finishedOn; byproducts are "Additional artifacts generated during the build that are not considered the output." (Source: https://slsa.dev/spec/v1.1/provenance, as of 2026-09-16.)

The vocabulary maps onto a receipt better than the schema does: instruments and versions are resolvedDependencies; policy, profile, and manifest are externalParameters (agent-controlled, verified downstream by signature); the tool build is builder.id and version; captured instrument output and SARIF files are byproducts; the evaluation clock is startedOn and finishedOn. Borrow the names, do not emit SLSA provenance: a receipt's subject is a snapshot being judged, not an artifact being built, and SLSA consumers would misread it.

### 3.4 Mapping a receipt and a ruling onto Statement plus a custom predicate

Receipt. subject is the snapshot: one ResourceDescriptor named "snapshot" with digest {"gitTree": "<tree id>", "sha256": "<inventory digest>"} and an annotation for the identity kind (working, staged, committed) that W24 keeps separate. predicateType is a Warrant URI (section 9). The predicate carries the policy as a ResourceDescriptor with the effective-policy digest; the inventory digest; instruments as {name, version, digest}; the tool build; evidence as ResourceDescriptors, each with its file digest, producing instrument, and the snapshot digest the evidence itself claims (section 8); the rulings relied on, by id and digest; the evaluation clock and its source; and the four facets with findings. Following the VSA, the policy lives in the predicate rather than as a second subject, so "what was verified" and "what it was verified against" cannot be confused by a consumer matching on digest.

Ruling. subject is what the ruling is about: the effective policy digest it ratifies or amends, and the candidate tree or trees it covers as gitTree digests. predicateType is one of a small set of URIs by kind (policy, exception, trust, override), so the kind is inside the signed bytes and the allowed-signers namespace can match it (section 4.2). The predicate carries id, supersedes, scope, the finding identity for exceptions, disposition, reason, signer principal, signed_at, expires_at, and candidate range. Rulings stay small and flat, because the signer reads them.

### 3.5 Can DSSE carry an sshsig?

Mechanically, yes. DSSE places "no restriction on the signature algorithm or format", so the PAE bytes can be the message given to ssh-keygen -Y sign and the resulting blob goes in signatures[].sig with the key fingerprint as keyid. Observed: a hand-built PAE for a Statement payload was signed on stdin with ssh-keygen and verified against an allowed-signers file. (Source: DSSE protocol.md; observed, OpenSSH 10.0p2, 2026-09-16.)

Practically, nothing else would understand it. cosign verify-blob takes --key as "path to the public key file, KMS URI or Kubernetes Secret" and cosign import-key-pair "Imports a PEM-encoded RSA or EC private key", neither an OpenSSH key; Rekor's dsse type parses verifiers only as x509 public keys (its entry code reports "could not parse public key as x509" otherwise), and the only Rekor type accepting an ssh signature is rekord, whose signature.format enum is pgp, minisign, x509, ssh. A verifier of a DSSE-wrapped sshsig would be Warrant plus ssh-keygen, the same as for a detached signature, with an extra base64 layer and a PAE prefix that stops stock ssh-keygen from checking the file directly. (Sources: https://github.com/sigstore/cosign/blob/main/doc/cosign_verify-blob.md; https://github.com/sigstore/cosign/blob/main/doc/cosign_import-key-pair.md; https://github.com/sigstore/rekor/blob/main/pkg/types/dsse/v0.0.1/entry.go; https://github.com/sigstore/rekor/blob/main/pkg/types/rekord/v0.0.1/rekord_v0_0_1_schema.json, as of 2026-09-16.)

Recommendation: a detached armored sshsig beside the canonical Statement. One stock command verifies it, the Statement stays readable, and the DSSE properties that matter are preserved another way: the sshsig namespace is the authenticated payload type, the embedded public key is the key hint, multiple signers are multiple .sig files, and no canonicalization happens at verification time. If a team later needs Sigstore or in-toto tooling to consume Warrant records, wrap the same Statement in a DSSE envelope signed by a cosign or KMS key at the gate; sshsig rulings remain the human approval channel and DSSE becomes the machine transport. DSSE's experimental "Signature Extensions" on the devel branch could carry an sshsig alongside a standard signature if that ever matters. (Source: https://github.com/secure-systems-lab/dsse/blob/devel/envelope.md, as of 2026-09-16.)

## 4. Trust roots and distribution

### 4.1 Where the allowed-signers file lives

The constraint is that an agent with repository write access cannot change the file the gate verifies against. Four placements were considered.

Outside the repository on the verifying machine. This is what git-config recommends as the ordinary case: "This file can be set to a location outside of the repository and every developer maintains their own trust store." For Warrant the gate reads it from a path such as ${XDG_CONFIG_HOME:-~/.config}/warrant/allowed_signers on the gate machine, or /etc/warrant/allowed_signers on a shared runner, owned by a user the agent is not. This satisfies HC5 directly and needs no new mechanism. (Source: git-config(1), as of 2026-09-16.)

A CI secret or protected file. On hosted CI the trust root is a repository or organization secret, or a file provisioned on a self-hosted runner, materialized to disk by the workflow before warrant gate runs. Agents that can push to the repository cannot read or change secrets unless they can also edit the workflow that reads them; that is why the workflow file must be covered by branch protection or a ruleset with bypass disabled (section 6).

A signed root-of-trust ruling in the repository. The repository carries warrant/allowed_signers as a documentation copy, and a ruling of kind trust, signed under a dedicated namespace by a key already in the gate's root, states the new root. The gate applies a trust ruling only after verifying it against the root it already holds, so the chain bootstraps from one out-of-band file and every later change is a signed, visible, supersedable record. git-config describes the in-repo variant for commits: "A repository that only allows signed commits can store the file in the repository itself ... This way only committers with an already valid key can add or change keys in the keyring." The same logic holds for Warrant, with the important caveat that the initial root still comes from outside. (Source: git-config(1), as of 2026-09-16.)

A git commit-signature chain. Trusting the file because the commit that last touched it was signed by a trusted key is the weakest option: it depends on every path to the file being covered by signed-commit enforcement, on the verifier walking history correctly, and on the "git" namespace, which section 1.4 argues Warrant should not share.

Recommendation for v1: the first two, with the third as the mechanism for change. The gate takes the trust root from a path it owns; the repository copy is documentation; changes to the root are trust rulings that the gate applies only after verifying against its current root and then writes back to its own copy.

### 4.2 Rotation, multiple signers, delegation, and expiry

Rotation is two lines in the file: the old key with valid-before and the new key with valid-after. Verification of an old ruling at its signed time still passes because the verifier supplies -Overify-time (section 5). Multiple signers are multiple lines; a threshold (two of three humans for a policy change) is Warrant logic over multiple .sig files, since ssh-keygen verifies one signature at a time. (Source: ssh-keygen(1), ALLOWED SIGNERS section, as of 2026-09-16.)

Delegation with scope is where the namespaces= option earns its keep. If Warrant uses one namespace per ruling kind, the trust root can say that a delegate may sign exceptions but not policy changes or trust changes, and ssh-keygen enforces it with no Warrant code:

```
trey@example.com namespaces="warrant-ruling-*@example.com,warrant-receipt@example.com" ssh-ed25519 AAAA...
delegate@example.com namespaces="warrant-ruling-exception@example.com",valid-before="20261231Z" ssh-ed25519 AAAA...
gate@ci.example.com namespaces="warrant-receipt@example.com" ssh-ed25519 AAAA...
```

The namespaces value is a pattern-list in the ssh_config(5) PATTERNS syntax, so wildcards work. Observed on this machine: a line with namespaces="warrant-ruling-*@example.com" verified a signature made under warrant-ruling-policy@example.com and rejected one made under warrant-other@example.com with "key is not permitted for use in signature namespace". (Observed, OpenSSH 10.0p2, 2026-09-16.) Delegation expiry is valid-before. Finer scope (a delegate may grant exceptions only for one module) is a field inside the ruling that the gate checks against the trust ruling that established the delegation. (Source: ssh-keygen(1), as of 2026-09-16.)

### 4.3 Emergency overrides that stay visible

An override is a ruling of kind override under its own namespace, with a single tree id as its candidate range, an expiry measured in hours or days, a reason, and no supersession of the rule it overrides. The receipt's approval facet reports "overridden" rather than "approved", the override's id appears in the receipt, and warrant gate exits zero only because the override is valid at gate time. Nothing about the override is hidden from a reader of the receipt, and it cannot be reused for the next candidate because the tree id differs.

## 5. Time

Three clocks appear in this design and they must not be confused: the time a ruling was signed (inside the signed bytes), the time an evaluation ran (the receipt's evaluation clock), and the time the gate decides (the trusted current time used for expiry).

Evaluation clock. The receipt records the clock at which the evaluation ran and where that clock came from ("system" or "override"). A --now flag lets a replay reuse a recorded clock so that a re-evaluation of the same snapshot under the same policy produces the same verdict and the same expiry decisions; the resulting receipt says its clock was overridden. This is the "record the clock, allow --now for replay" pattern W16 asks for.

Trusted time at the gate. The integration gate evaluates ruling expiry, delegation expiry, and key validity windows against its own clock, never against a time found in the candidate. A lane run on an agent's machine may use --now for reproduction; the gate refuses --now unless an operator flag says the run is a replay, and a replay receipt is not a gate receipt. Two things are checked against different times: the ruling's own expires_at is checked against the gate clock, and the signing key's validity window is checked at the ruling's signed_at, which is the behavior git implements by passing -Overify-time= the payload timestamp. The difference from git is that Warrant should not let the drafter set signed_at: the signing command fills it from the signer's clock immediately before canonicalizing and signing, so the value is in the signed bytes and was chosen by the signer's machine. (Sources: ssh-keygen(1) -O verify-time; https://github.com/git/git/blob/master/gpg-interface.c, as of 2026-09-16.)

RFC 3161. A Time-Stamp Authority "provides a 'proof-of-existence' for this particular datum at an instant in time", and the canonical use is "to verify that a digital signature was applied to a message before the corresponding certificate was revoked". Sigstore operates a public TSA at https://timestamp.sigstore.dev/api/v1/timestamp, and cosign verify-blob has --use-signed-timestamps to "verify rfc3161 timestamps". (Sources: https://www.rfc-editor.org/rfc/rfc3161, Section 1; https://docs.sigstore.dev/cosign/verifying/timestamps/; https://github.com/sigstore/cosign/blob/main/doc/cosign_verify-blob.md, as of 2026-09-16.)

Is a TSA worth it for Warrant? For v1, no. The gate is already a trusted party with a clock, the signer's clock sets signed_at, and the threat a TSA addresses (proving a signature predates a key revocation without trusting either party's clock) only matters after a key compromise, when the sensible response is to re-sign the rulings that still matter with the new key. For a team that must show an auditor when a ruling was signed, an optional field in the signature index holding an RFC 3161 token over the .sig file is a clean later addition and does not change the record format.

## 6. Alternatives compared

GPG and OpenPGP. Git's default. gpg 2.4.7 offers trust models (pgp, classic, tofu, tofu+pgp, direct, always, auto), key expiry, revocation certificates, and --assert-signer, which "checks whether at least one valid signature on a file has" been made by a named signer. The cost is keyring and agent state, the web-of-trust vocabulary, and the fact that the estate's humans have SSH keys and no OpenPGP keys. Nothing in Warrant's requirements needs OpenPGP's certification graph. (Source: gpg(1), GnuPG 2.4.7, as of 2026-09-16.)

minisign and signify. minisign is "a dead simple tool to sign files and verify signatures" using Ed25519 (minisign -Sm file to sign, minisign -Vm file -P <pubkey> to verify) with a trusted comment that "can be used to add ... the intended file name, timestamps, resource identifiers, or version numbers to prevent downgrade attacks." signify has the same key format (signify -S -s seckey -m message; signify -V -p pubkey -m message) and passphrase-protects secret keys by default; Rekor's rekord type accepts minisign. Both lack a verifier-enforced namespace, a trust-list file with per-key scopes and validity windows, agent integration, and any relation to the signer's git key. (Sources: https://jedisct1.github.io/minisign/; https://man.openbsd.org/signify, as of 2026-09-16.)

Sigstore keyless, Fulcio, Rekor, gitsign. "Fulcio issues short-lived certificates binding an ephemeral key to an OpenID Connect identity"; Rekor is "an immutable, append-only ledger" holding "the artifact's digest, signature and certificate"; "Sigstore services never obtain your private key." Blob signing is cosign sign-blob <file> --bundle bundle.sigstore.json; verification is cosign verify-blob --bundle <path> --certificate-identity <identity> --certificate-oidc-issuer <issuer> <blob>, and "Either --certificate-identity or --certificate-identity-regexp must be set for keyless flows." --insecure-ignore-tlog covers artifacts not in the log, which "cannot be publicly verified." gitsign does the same for commits (git config gpg.x509.program gitsign; gpg.format x509) with certificates "valid for approximately 10 minutes", and "GitHub doesn't recognize Gitsign signatures as verified at the moment." (Sources: https://docs.sigstore.dev/about/overview/; https://github.com/sigstore/cosign/blob/main/doc/cosign_sign-blob.md; https://github.com/sigstore/cosign/blob/main/doc/cosign_verify-blob.md; https://github.com/sigstore/gitsign, as of 2026-09-16.)

For a solo maintainer, keyless means an interactive OIDC login per signature, a network dependency at signing time, a public log entry per ruling (digests and identity, not content), and a TUF-distributed root to verify anything; none of that buys more than a passphrase-protected key on a separate Unix user. For a company, the identity binding fits the gate principal well: a GitHub Actions workflow can sign receipts with its workflow identity and a consumer verifies with --certificate-oidc-issuer https://token.actions.githubusercontent.com. It fits human rulings poorly, because a browser login is not a considered act of approval in the way a passphrase prompt over a rendered record is.

age. "A simple, modern and secure file encryption tool"; no signature function. Not applicable. (Source: https://github.com/FiloSottile/age, as of 2026-09-16.)

Signed commits or tags as the approval channel. A commit signature proves that the key holder created that commit object (tree, parents, author and committer lines, message); a tag signature does the same for a tag object. Neither carries a policy digest, scope, expiry, supersession pointer, or finding identity unless the human types them into the message, and both use the "git" namespace that every git verifier accepts. That is the W15 gap: the signature the human makes forty times a day to record edits would also be the signature that approves a rule change, and nothing distinguishes the two acts. A signed tag whose message is a canonical ruling is a workable transport, but it approves one commit rather than a policy digest across a candidate range, and a verifier must parse the message to learn what was approved. Signed commits still answer "who wrote this", and the two coexist on one key with two trust lines. (Sources: https://git-scm.com/docs/gitformat-signature; git-config(1); observed, 2026-09-16.)

GitHub branch protection and rulesets. Branch protection can require "a specific number of approving reviews", "dismiss stale pull request approvals when commits are pushed that affect the diff", require that "the most recent reviewable push must be approved by someone other than the person who pushed it", require signed commits and status checks; by default restrictions "do not apply to people with admin permissions" unless bypass is disabled. Rulesets aggregate so that "the most restrictive version of the rule applies", carry bypass lists, and are visible to "Anyone with read access to a repository". This is the right enforcement for a team: an agent account without bypass cannot merge past a required review or check. It is not the approval record: a review approves a diff as GitHub computes it, lives in GitHub's database, cannot be verified offline, and cannot be scoped to one finding until one date. (Sources: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches; https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets, as of 2026-09-16.)

Recommendation. v1: sshsig with per-kind namespaces, an allowed-signers root outside the candidate, detached signatures over JCS-canonical Statements, and gate receipts signed by a gate key under a receipt namespace; everything verifies with ssh-keygen plus a small amount of Warrant logic, offline. Teams add, as needed: an SSH CA with cert-authority in the root; branch protection or a ruleset that makes warrant gate a required check, protects the workflow file, and denies bypass to agent accounts; and optionally keyless Sigstore signing of gate receipts wrapped in DSSE for organizations that already verify Sigstore bundles.

## 7. Threat model

Principals. The agent has full write access to the working tree and runs any command as its own Unix user: it can edit or delete anything under warrant/, including rulings, signatures, and the documentation copy of allowed_signers; commit with any author, committer, and date; set git configuration; generate its own keys and sign anything with them; shadow warrant or ssh-keygen on its own PATH; write any JSON that looks like a receipt; and draft a ruling whose summary misleads the signer. The signer holds a passphrase-protected key in a home the agent cannot read, on a different Unix user. The gate runs where the agent has no write access, with a trust root from outside the candidate and a binary it installed itself.

What each attack meets:

- Editing a ruling or its signature. Any byte change fails verification ("incorrect signature", observed). Deleting a ruling removes an approval, which only makes a verdict stricter; a deleted trust ruling is detected because the gate's own root copy no longer matches the signed lineage. Renaming changes nothing verified; the id is in the signed bytes.
- Editing the in-repo allowed_signers. No effect; the gate does not read it (HC5). Warrant should diff the repository copy against the gate's root and report a mismatch as a signal so tampering is visible rather than silently ignored.
- Replaying an old signature. A ruling binds a policy digest, a candidate range, an id, an expiry, and a supersession pointer inside the signed bytes. Against a new policy it fails the digest check; against a new tree, the range check; after expiry, the clock check; after supersession, the newer record wins. A ruling copied from another repository fails on both digests. What signatures do not prevent is reuse of a ruling still valid on its own terms; range and expiry are the controls.
- Forging a receipt. An unsigned receipt is a claim. A gate receipt is signed by the gate key under the receipt namespace, and consumers accept only receipts that verify against the gate principal; lane receipts are advisory and say so. Independently, warrant verify recomputes every binding it can (section 8), so a receipt naming a tree that does not exist or an evidence digest that does not match fails without any key.
- Editing the tool binary. On the agent's machine nothing prevents it, which is why lane results are advisory. On the gate machine the binary comes from a release whose digest the operator verified (W25), the receipt records the build digest, and the verifier compares it to its pinned value.
- Backdating. Git checks key validity at the commit's own timestamp, which the committer sets. Warrant's signing command sets signed_at from the signer's clock, and the gate checks expiry against its own clock, so an agent cannot age a ruling into a window where a revoked key was valid or out of one where the ruling has expired.
- Reaching the signer's key. File permissions, the passphrase, and an agent socket readable only by its owner are the controls, with ssh-agent(1)'s forwarding warning in mind. A confirmation-required identity or an ed25519-sk key adds a human action per signature, which defeats a stolen socket. This lives in documentation, not code.
- Misleading the signer. A draft can be accurate and still hide a widening. The signing command must render the effect summary from the canonical bytes it is about to sign, never from agent-supplied text, and show the W14 widening classification beside the prompt. The signer's obligation to read is the residual risk; no mechanism here removes it.

What the gate must do on a trusted runner: read the trust root from its own path; compute the snapshot identity itself; verify every ruling it relies on with ssh-keygen against that root, under the namespace for the ruling's kind, with the principal named in the ruling, verify-time set to the ruling's signed_at, and the revocation list; check the ruling's policy digest against the effective policy it compiled, its candidate range against the tree under evaluation, its expiry against the gate clock, and its supersession state; verify every evidence file's digest and the snapshot digest inside the evidence; run the evaluation with the pinned binary and instruments; write the receipt atomically and sign it with the gate key.

"Cryptographic integrity does not establish a trustworthy signer or a truthful test" means, in practice: a valid signature proves that whoever controlled the key at signing time signed exactly these bytes, not that the person read them, understood the summary, was not deceived, or still controls the key. A test receipt whose digest matches proves that the named runner emitted that output for that tree, not that the test asserts anything, that its assertions were not weakened in the same candidate (W14's "weakened assertions" signal exists for this), or that the runner ran what it was asked to. Warrant's honest claim is narrower than "approved and tested": this exact record was signed by this key under this namespace, and this exact evidence was bound to this exact snapshot. The four facets keep the narrow claim from being read as the broad one.

## 8. Receipt verification algorithm sketch

Inputs: the receipt bytes and, if present, its .sig; the trust root and revocation list; the pinned tool build digest; access to the repository object store or the snapshot; the evidence files the receipt names; the rulings it names; a clock (system, or --now for replay).

Steps:

1. Parse the receipt as a Statement. Reject if _type, subject, or predicateType is missing or unknown, or if the file is not byte-identical to its JCS canonical form (a cheap tamper and tooling check).
2. If a .sig exists, verify it with ssh-keygen -Y verify under the receipt namespace against the trust root, with -I set to the receipt's gate principal. Record the outcome as "signed by gate", "unsigned", or "signature invalid"; an invalid signature is a failure, an unsigned receipt is advisory.
3. Resolve the subject. "The same snapshot" means the same gitTree id under the same object format, and, when the receipt carries Warrant's own sha256 inventory digest, the same inventory digest recomputed from the tree's blobs and paths. A working-tree snapshot is a tree Warrant wrote itself (a temporary index over the working tree, including untracked and modified files, excluding ignored ones); its identity kind is recorded, and a receipt over a working-tree identity never satisfies a gate that asked for a committed identity. If the tree cannot be found, fail with "snapshot unavailable"; do not fall back to path comparison.
4. Compare the tool build and every instrument entry to the pinned values. A mismatch is "instrument drift", a failure for gate receipts and a warning for lane receipts.
5. For each ruling named: locate it by id, check its digest against the receipt's record of it, then run the ruling verification described in section 7. A ruling that verified when the receipt was made but has since expired or been revoked makes the receipt "stale", which is a distinct outcome from "invalid".
6. For each evidence entry: recompute the file digest and compare; parse the evidence's own Statement (for a test receipt, the test-result Statement; for SARIF, the run's artifact and version-control provenance fields) and require that its declared subject digest equals the receipt's snapshot digest. Evidence with no subject digest is "unbound evidence", which W24 says is a failure, never a warning.
7. Recompute the policy digest from the policy files at the snapshot and compare it to the receipt's policy descriptor.
8. Report: integrity (bytes and signature), snapshot match, instrument match, rulings state, evidence state, policy match, and the clock used. Exit zero only if every check passed and the receipt is a gate receipt; exit codes distinguish "invalid", "stale", "advisory", and "unavailable".

Failure modes worth naming in the output: a receipt whose canonical form differs from its bytes (someone reformatted it); a snapshot that exists but under a different object format (SHA-1 versus SHA-256 repositories yield different gitTree ids for identical content); an evidence file whose digest matches but whose own subject names a different tree (the test ran against something else); and a receipt from a build whose digest is not pinned (an agent-modified binary).

How external evidence is bound: the receipt records the evidence file's digest, the instrument that produced it, and the snapshot the evidence claims. For test runners, Warrant should require the runner adapter to emit a Statement with the test-result predicate and the tree id as subject, and to sign nothing; the binding is the digest chain, not a signature. For SARIF files, the adapter records the SARIF file's sha256 and extracts the run's tool name and version to compare with instruments.lock. An evidence file that was produced for a different tree can be digest-perfect and still fail step 6; that is the point of step 6.

## 9. Recommendations for the spec

Concrete choices, in the order the spec would state them.

Record format. Every ruling and receipt is an in-toto Statement v1 (_type "https://in-toto.io/Statement/v1") with a Warrant predicateType, serialized as RFC 8785 canonical JSON, UTF-8, no trailing newline, written once with the atomic rename HC14 already requires. Signed records contain no floats; counts are integers, everything else is a string. Verification compares bytes and never re-serializes.

Predicate types. One URI per kind, versioned by major number in the URI: .../receipt/v1, .../ruling/policy/v1, .../ruling/exception/v1, .../ruling/trust/v1, .../ruling/override/v1. The kind is therefore inside the signed bytes and cannot be relabeled after signing.

Signature carrier. A detached, armored sshsig file beside the record: <name>.json and <name>.json.sig. Multiple signers are <name>.json.<principal-or-fingerprint>.sig files; a threshold is Warrant logic. No DSSE in v1; a DSSE wrapper for teams is documented as a transport that carries the same Statement.

Namespaces. One per kind, following the ssh-keygen NAMESPACE@YOUR.DOMAIN convention: warrant-ruling-policy@<domain>, warrant-ruling-exception@<domain>, warrant-ruling-trust@<domain>, warrant-ruling-override@<domain>, warrant-receipt@<domain>. Trust lines use the namespaces= pattern list to scope people to kinds, and warrant-ruling-*@<domain> for a full signer. The <domain> is an open decision below.

Trust root. The gate reads ${XDG_CONFIG_HOME:-~/.config}/warrant/allowed_signers (or a path from its config) plus an optional revoked_keys file in the same directory, both owned by the gate user. The repository's warrant/allowed_signers is documentation; a mismatch between it and the gate's root is a reported signal. Changes to the root are trust rulings, applied only after verifying against the current root. Hosted CI materializes the root from a secret before the gate runs, and the workflow file is protected by a ruleset with bypass denied to agent accounts.

Commands and what they run. warrant rule sign <file> fills signed_at from the local clock, canonicalizes, renders the effect summary from the canonical bytes, shows the widening classification, then runs ssh-keygen -Y sign -n <namespace-for-kind> -f <key> on stdin and writes the .sig. warrant rule verify <file> runs ssh-keygen -Y find-principals to learn the principal, requires that it equal the ruling's signer field, then ssh-keygen -Y verify -f <root> -I <principal> -n <namespace> -s <sig> -r <revoked> -O verify-time=<signed_at in YYYYMMDDHHMMSSZ> (the 14-digit form with a Z suffix was accepted on this machine, OpenSSH 10.0p2, 2026-09-16), and then checks digest, range, expiry, and supersession. warrant gate signs its receipt with the gate key under warrant-receipt@<domain>. warrant verify <receipt> runs the algorithm in section 8 and is dependency-light: ssh-keygen, git object access, and a hash.

Time. Receipts carry evaluated_at and clock_source. --now is accepted by lane commands and by warrant verify for replay; the gate refuses it except under an explicit replay flag whose receipts are marked advisory. Ruling expiry is checked against the gate clock; key validity is checked at the ruling's signed_at. No RFC 3161 in v1; an optional timestamp token field is reserved in the signature index for later.

Snapshot identity. subject.digest carries gitTree plus Warrant's own sha256 inventory digest, with an annotation naming the identity kind (working, staged, committed). Two snapshots are the same when both digests match; the object format (SHA-1 or SHA-256 repository) is recorded because the gitTree id depends on it.

Compatibility floor. OpenSSH 9.1 or newer on the gate for -O verify-time with the Z suffix; git 2.34 or newer if signed commits are used alongside, the floor GitHub states for SSH signature verification (https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification, as of 2026-09-16); document both.

Open decisions for the maintainer:

1. The namespace domain. Signatures embed it, so changing it later means re-signing or accepting two namespaces in the verifier. A domain the project controls is the safe choice; a project name without a domain is allowed by ssh-keygen but collides more easily.
2. Whether lane receipts are ever signed. Signing them with the agent's own key adds nothing the gate trusts, but it does let a human tell which agent produced which lane receipt. Cheap, and worth deciding once.
3. Threshold signing for policy rulings. One signer is right for the estate today; the record and file layout above allow two-of-n without change, but the trust ruling format needs a field for the required count if teams are to use it.
4. Whether check-novalidate output is shown to humans before trust is decided. It is useful for "who signed this" in an untrusted context and dangerous if any reader mistakes it for verification. The recommendation is to show it only inside warrant rule explain, with the word "unverified" in the output.
5. Whether to require ed25519-sk or a confirmation-required agent identity for the trust and override kinds. This is a policy for signers, not a code change, but the spec should say whether Warrant checks the key type in the trust root for those namespaces.
6. The identity kind rule for evidence. Whether a test receipt over a working-tree snapshot may ever satisfy an obligation at the gate, or whether the gate re-runs every required instrument on the committed candidate. The vision's "evaluated fresh" language argues for the second, at a cost the spec should state.

## Sources

All fetched or read on 2026-09-16.

- OpenSSH PROTOCOL.sshsig: https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.sshsig
- ssh-keygen(1), ssh-add(1), ssh-agent(1): OpenSSH 10.0p2 manual pages on this machine; https://man.openbsd.org/ssh-keygen.1
- OpenSSH release notes: https://www.openssh.com/releasenotes.html
- git-config(1), git-verify-commit(1): git 2.47.3 manual pages; https://git-scm.com/docs/git-config
- gitformat-signature: https://git-scm.com/docs/gitformat-signature
- git gpg-interface.c: https://github.com/git/git/blob/master/gpg-interface.c
- RFC 8785, JSON Canonicalization Scheme: https://www.rfc-editor.org/rfc/rfc8785
- RFC 3161, Time-Stamp Protocol: https://www.rfc-editor.org/rfc/rfc3161
- in-toto Attestation Framework v1: https://github.com/in-toto/attestation/tree/main/spec/v1 (statement.md, envelope.md, predicate.md, resource_descriptor.md, digest_set.md, README.md, bundle.md) and https://github.com/in-toto/attestation/blob/main/spec/predicates/test-result.md
- DSSE: https://github.com/secure-systems-lab/dsse/blob/master/protocol.md and https://github.com/secure-systems-lab/dsse/blob/master/envelope.md; experimental extensions at https://github.com/secure-systems-lab/dsse/blob/devel/envelope.md
- SLSA Provenance v1: https://slsa.dev/spec/v1.1/provenance; Verification Summary: https://slsa.dev/spec/v1.1/verification_summary
- Sigstore: https://docs.sigstore.dev/about/overview/; https://docs.sigstore.dev/cosign/signing/overview/; https://docs.sigstore.dev/cosign/verifying/timestamps/; cosign reference https://github.com/sigstore/cosign/blob/main/doc/cosign_sign-blob.md, cosign_verify-blob.md, cosign_import-key-pair.md; Rekor rekord schema https://github.com/sigstore/rekor/blob/main/pkg/types/rekord/v0.0.1/rekord_v0_0_1_schema.json and dsse entry https://github.com/sigstore/rekor/blob/main/pkg/types/dsse/v0.0.1/entry.go; gitsign https://github.com/sigstore/gitsign
- minisign: https://jedisct1.github.io/minisign/; signify: https://man.openbsd.org/signify; age: https://github.com/FiloSottile/age
- GnuPG: gpg(1) manual page, GnuPG 2.4.7 on this machine
- GitHub docs: about commit signature verification, adding a new SSH key, telling git about your signing key, about protected branches, about rulesets (URLs in the text)
- Warrant inputs: docs/design/2026-09-16-specgate-vision.md and docs/design/2026-09-16-agent-builder-wishlist.md (W14, W15, W16, W24) in the Specgate repository; docs/specs/2026-09-16-warrant-v1-spec.md sections 3.5 and 3.7 in the Warrant repository
