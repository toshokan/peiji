# peiji

A bucket-wise quota service.

## Usage

You should use the included Dockerfile (and images generated by it) to run this application.

The following environment variables must be set:
1. `REDIS_URI` should point to a Redis instance, for example: `redis://redis-instance`
1. `CONFIG_FILE_PATH` should be the path to an accessible TOML config file, for example: `/app/config.toml`

The application will expose an HTTP service on TCP port 80 on all interfaces.

## Configuration

The configuration file should describe the quota limits for each bucket.

The following example config file will allow:
- the `bkt::test-client::global` bucket to be charged 15 times per hour.
- the `bkt::test-client::service-a::read` bucket to be charged 50 times per day.
- the `bkt::other-client::global` bucket to be charged 1250 times per month.

```toml
[[limits]]
bucket = "bkt::test-client::global"
type = "hourly"
freq = 15

[[limits]]
bucket = "bkt::test-client::service-a::read"
type = "daily"
freq = 50

[[limits]]
bucket = "bkt::other-client::global"
type = "monthly"
freq = 1250
```

The available types are `secondly`, `minutely`, `hourly`, `daily`, `weekly`, and `monthly`. 
Note that a `monthly` quota period is always exactly 30 days.

## API

The service exposes the following route:
```
POST /api/v1/charges
```

An `application/json` payload is expected with an array of charges, for example:
```json
[
  {
    "bucket": "bkt::test-client::global",
    "cost": 1
  },
  {
    "bucket": "bkt::test-client::service-a::read",
    "cost": 3
  },
]
```
In a single request, at most 100 buckets may be charged and no bucket may be charged more than 1000 cost.

The application will respond with one of the following string JSON string payloads:

- `ok` means the charges were applied and there is still available quota in each bucket.
- `slow_down` means the charges were applied, and there is still available quota in each bucket, but at least one bucket is over 90% full for the current period.
- `stop` means the bucket is already full for the current period. The bucket will be blocked for a few seconds to throttle the owner.
- `block` means a request for a bucket is being made after the owner was already warned to `stop`. The bucket is blocked for (at least) one minute. 
