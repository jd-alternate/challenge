# Challenge

I didn't realise this writeup would end up being so long so feel free not to read it, but it captures my decisions, challenges, and assumptions when creating this project.

## Code Structure

I decided to split up my code into three parts:

- the model, which contains the Event, Client, and Transaction structs
- formatting, which handles reading and writing the data.
- the system, which contains the business logic

The important split is between the system and the formatting: I don't want my business logic having any dependence on the formatting, because if we later want to be able to read from other formats like JSON, I shouldn't have to change my business logic code. So my business logic code just accepts an interator of Events and processes them, then returns the resultant clients. It's up to the formatting code to work out how to construct such an iterator based on the input. Using an iterator makes for slightly hairier function signatures, but allows us to stream data from the input without having to load it all into memory first.

Some more detail on each of the three parts:

## Modelling

### Type Aliases

I've defined some type aliases: ClientID, TransactionID, and Amount. These exist so that it's easier to follow the code, but also so that it's easier to switch from one type to another. For example, if we end up with way more transactions and need to use a larger integer type for that, we only need to update one place.

### Amounts

Given that we need to support decimal values up to 4 decimal places, I went with an external crate which handles decimals: rust_decimal. Instantiating decimal values is easy enough with a macro and mathematical operations all work as per normal out of the box. That crate uses 128 bit integers under the hood which some bits dedicated to the fractional part of a number, which should be more than enough for our purposes. If we ever need to go higher, for example to support some cryptocurrencies that have extremely small base units (like Ethereum's wei), we could consider switching to something like BigDecimal which uses heap-allocated numbers of arbitrary precision (but that's more expensive and I doubt even Ethereum needs that).

### Naming

Although the spec describes the input CSV as one transaction per row, I decided to call those rows 'events', if only to free up the word 'transactions' for deposits and withdrawals, which actually have transaction IDs. The other events (disputes, resolves, chargebacks) lack a transaction ID and only act upon other transactions, which makes me feel like they're not deserving of the term. I should say, though, that I don't actually know what the industry terminology is so this is something I'd talk through in a real world situation.

### Events

Originally my Event enum had all five events as separate members, with shared fields being duplicated in each. Now I've got two enum members: transactions and dispute steps, each with a kind enum to further specify what kind of event it is. Both of these options have pros and cons: the DRYer approach expresses the taxonomy of events more clearly, by grouping transactions and dispute steps separately, however it's a little more clunky to instantiate a given enum value because you need to specify two 'kinds' e.g. transaction and deposit, whereas with the DRYer approach you could just specify that it's a deposit. You could write little convenience functions to get around that but it would be boilerplatey. As with all DRY vs non-DRY debacles, there's a risk that some new requirement comes along that puts pressure on your abstraction (e.g. for a contrived example a chargeback event may also specify some additional chargeback amount). In that case you could just chuck the extra info in the DisputeStepKind enum chargeback member but it might still be messier than just having a flat enum of the different events.

### Clients and Transactions

Unlike Events which are immutable, the Client and Transaction structs are mutable, so I decided to lock them down by having all fields being private and providing getters/setters. This is boilerplatey, and arguably unnecessary given this is application code, not library code, but it has two main benefits:

- invariants can be enforced: for example, the only way a client can be locked is through a chargeback
- it's clear which fields are read-only after instantiation (e.g. the client_id field on Transactions)

One downside of locking down my fields is that it's harder to write assertions in tests without providing a constructor function that lets you build a client with the exact fields you want to assert on. With more time I'd look into structuring the code such that those tests have permission to just create Client structs directly without the additional ceremony, by virtue of where they sit in the module hierarchy.

I should mention that the approach I've taken with Client and Transaction is fairly object-oriented (in the sense that we're colocating data with methods that act on that data); a pattern which has declined in favour over time. At any rate, I think in this case it results in fairly readable code, though I'd be interested in exploring more functional alternatives.

### Storing IDs

One decision I've made which I'm most willing to revisit is to not store IDs on my Clients or Transactions. The reason being is that my system's current state is represented by a couple of HashMaps mapping from IDs to Clients/Transactions, and if my Clients/Transactions themselves also contain IDs, it allows for impossible states to arise (e.g. when my HashMap points from client ID 5 to a Client who themselves says their ID is 10). The downside of this approach is that I can't include IDs in error messages from within those structs, and I have to do a bit of a dance to map from my HashMap of clients to my resultant CSV report because I can't just grab the Client's ID from within the Client itself. On net it's probably worth just including the IDs in the structs, impossible states be damned, but in the current state of the code it's not causing too much trouble.

## Formatting

### Allowing for future forms of formatting

At the moment, the formatting side of things is fairly simple with only a single csv option, so it's arguably overkill that we even have that folder there. But it makes it trivially easy to add other formats in the future. I haven't gone so far as to actually have a trait for reading/writing data, with a csv implementation, just because I think that actually _is_ overkill for the current implementation.

### Serde

I'm using serde to map from the structs to csv (and vice versa), but given there's no one-to-one mapping between say Client fields and what we want in the CSV (for example, there's no `available` field because that's derived from `total` and `held`, and I'm not aware of how to have serde call methods), I'm defining my own CSV variants of the structs to act as an intermediary. In the context of outputting the CSV report, this is more convoluted (and less efficient) than just having a function which maps from a Client to a CSV row, but one of the nice things is that I don't need to ensure that the CSV headers and the struct fields are kept in-sync, because I get that from serde for free. I'm not quite sure which approach I prefer, but I've stuck for the intermediary-struct approach just because it works well enough.

One snag I hit was in deserializing our amounts, because I'm using the rust_decimal crate for those and although that crate provides a custom serde deserializer, it doens't play nice with empty strings, which we encounter e.g. with Dispute events. For empty strings, I want that serialized into a None option value, but writing a custom deserializer for that proved quite hairy and so I ended up falling back to simply having serde deserialize the amount as a String so that I could then manually parse it into a Decimal afterwards.

## The System

I've got a function for processing events which takes the Events iterator and returns the resultant clients. It just so happens to make use of a Processor struct which maintains the state of clients/transactions and processes each event, but that's an implementation detail so I'm only testing that struct indirectly via the original function.

### Storage of state

Given that we could be given any client ID and any transaction ID in an event, I decided against using an array to store my data because it might be too sparse and memory intensive. With that said, it would be interesting to do some benchmarking and see how we'd go using an array of 2^16 elements for the clients, whose ids are only 16 bits.

### Assumptions

In terms of business logic, I've made some assumptions that weren't clear from the spec.

#### Locked status

I'm assuming that when a client is locked they can no longer deposit or withdraw funds, however existing transactions can still be disputed.

#### Failed deposits/withdrawals

I'm assuming that if a deposit or withdrawal fails (either due to the client being locked or due to insufficient funds) we don't actually store that transaction. The fact that these events come through with transaction IDs makes me hesitate to implement the logic this way, but I imagine for example that my ATM doesn't actually record a withdrawal transaction if there's insufficient funds.

#### Mismatched clients

Dispute events contain both a transaction ID and a client ID but the transaction itself also contains a client ID. I assume that if an event comes through where those two client IDs are different, there's been some mistake, and so I'm failing that event.

#### Holding funds

The spec describes how the held amount changes for the different dispute steps but seems to only consider the case of a deposit. For example, I'm told that if we're disputing money, we need to hold the disputed amount. But intuitively it seems like if we're holding a positive amount for a deposit, we should be holding a negative amount for a withdrawal. This leads to strange behaviour where a withdrawal can be disputed, leading to the client immediately having more available funds, which seems wrong. I'm not sure if only deposits should be disputable. At the moment I've got withdrawals being disputable and holding a negative amount of money, but this is almost certainly not what we want.

## Testing

I've got unit tests for both the system and the formatting code, however I've chosen not to test the Client, Transaction, or Processor structs directly, simply because I consider the logic contained within those to be implementation details that could be refactored to live somewhere else, and I don't want to have to rewrite tests in that case.

I've got a couple of integration tests that use the assert_cmd crate to actually run the binary against a real file created in a temp directory, just to ensure that the end-to-end works, but given that they run slower than the unit tests, there aren't many.
