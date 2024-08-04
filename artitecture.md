# Analysis on Artitecture of Fedimint

Fedimint employs three core components: `Chaumian eCash`, `federations`, and `Lightning swaps`. `Chaumian eCash` provides privacy
by issuing and redeeming IOU notes representing claims on Bitcoin, ensuring anonymity through blind signatures. This means
that even the mint can't track the ownership of the eCash notes, which helps preserve user privacy.
`Federations` in Fedimint involve multiple guardians who collectively operate the mint. This distributed control prevents a
single point of failure and reduces the risk of corruption or theft. It also enhances redundancy, ensuring that transactions
can proceed even if some guardians are offline, provided a quorum is reached.
`Lightning swaps` integrate with the Lightning Network, enabling users to swap Fedimint Bitcoin for Lightning Bitcoin and vice
versa. This integration uses Hash Time Lock Contracts to secure transactions across both networks, providing seamless and secure
interoperability.

# Analyzing Transactions and Schema

### Fedimint Transactions Overview:

- Intramint Payments: Transactions that happen entirely within one Fedimint, involving the exchange or redemption of eCash notes.

- Fedimint to Lightning: Payments made from Fedimint to the Lightning Network, incentivizing Lightning Gateways to process payments.

- Fedimint to Fedimint: Payments between different Fedimints routed over the Lightning Network.

- Lightning to Fedimint: Payments coming from the Lightning Network to a Fedimint.

### Schema Assumptions:

- transaction_outputs table likely records outputs related to Fedimint operations, such as minting new tokens.

- transaction_inputs table likely records inputs related to Fedimint operations, such as burning tokens.

- kind field is used to categorize transactions as related to minting ('mint'), wallet ('wallet') or lightning ('ln') or possibly other types.
