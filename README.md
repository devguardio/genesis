devguard genesis
================


individual system settings at scale.

### cli usage

from the CLI simply call
```bash
carrier genesis <identity>
```

- your local editor will open (set with the EDITOR environment variable, like git).
- edit the configuration and then exit your editor.
- enter a commit message and hit enter
- the device will attempt to settle on the new configuration or revert if it can no longer connect to the carrier ring



### conduit usage

to manage configurations at scale, we recommend some sort of database with sha256 hashed config files.
you can also generate configs on the fly, but ensure the output yields predictable hashes.

When a new device publishes in conduit, request its current configuration hash, and match it against your database.
On mismatch, push the new config.

Carrier devices are stateless with no history of changes.
You must detect when a device reverts to its previous configuration, to avoid pushing a broken configuration in a loop.
One way would be to hold a previous_hash in the database, and mark the current configuration as failed,
if the device is rediscovered with the same configuration.


the endpoint uses this protobuf https://raw.githubusercontent.com/devguardio/carrier/master/proto/genesis.v1.proto
and requires only the :method header:

- HEAD will return GenesisCurrent but data remaining empty. use this to check if the devices configuration hash matches your conduits database
- GET will return GenesisCurrent
- POST expects a GenesisUpdate message and returns nothing, use this to update to a new configuration.

