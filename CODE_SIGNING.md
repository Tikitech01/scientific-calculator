# Code signing policy

Scientific Calculator Studio Pro is created and maintained by Sope Technology. The source code and release build instructions are published in this repository.

We are applying for free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/). If approved, Windows will show **SignPath Foundation** as the verified signer of standalone releases built from this repository. It will not identify Sope Technology as the certificate holder. Standalone Windows installers remain unsigned until a signed release is published; the Microsoft Store distributes its own signed package.

## Release controls

- Committer and reviewer: [Rakesh (`tiki-tech`)](https://github.com/tiki-tech).
- Signing approver: [Rakesh (`tiki-tech`)](https://github.com/tiki-tech).
- Release binaries must be built from the public source with a GitHub-hosted workflow. Signing requests require manual approval by the signing approver.
- Release artifacts should have matching product and version metadata. Sign the app executable and installer, then timestamp and verify both signatures before publication.

## Privacy

Calculations and saved sheets stay on the user's device. The app does not send them to a server. Website, update, and feedback links open only when the user selects them.
