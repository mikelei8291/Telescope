# Design

## Commands

- `/sub <url>`: subscribe to the live stream from the specified URL
- `/del <url>`: delete the subscription to the live stream of the specified URL
- `/list`: list all subscriptions of the current user
- `/platform`: list all supported platforms

## Database

- subs (HASH): `[platform:user_id:username -> live_id, ...]`
- platform:user_id:username (HASH): `[Telegram_user_id -> msg_id, ...]`
- Telegram_user_id (SET): `[platform:user_id_username, ...]`

## Workflows

- Add subscription
  ```mermaid
  flowchart LR
    subgraph s1["Add subscription"]
      n1["Add pair Platform:user_id:username to the<br>hash 'subs' with empty string as value"] --> n2["Add Telegram user ID to the hash<br>named with the pair and 0 as value"]
      n2 --> n3(["Add the pair to the set named<br>with the Telegram user ID"])
    end
    A["/sub &lt;url&gt;"] --> B{"Parse URL"}
    B -- Supported Service --> C{"Parse username"}
    B -- Unsupported Service --> D(["fa:fa-message Send failure message"])
    C -- Not Subscribed --> E["fa:fa-message Send confirmation message"]
    C -- Already Subscribed --> D
    E --> F{"Get<br>inline button<br>response"}
    F -- Confirmed --> s1
    s1 --> H(["fa:fa-message Change to success message"])
    F -- Cancelled --> I(["fa:fa-message Change to cancelled message"])
  ```
- Delete subscription
  ```mermaid
  flowchart LR
    subgraph s1["Delete subscription"]
      n1["Remove the Platform:user_id:username pair<br>from the set named with the Telegram user ID"]
      n1 --> n2["Remove the Telegram user ID from<br>the hash named with the pair"]
      n2 --> n3{"Does the hash<br>Platform:user_id:username<br>used to only have 1<br>Telegram user ID?"}
      n3 -- Yes --> n4["Remove the pair from the 'subs' hash"]
      n3 -- No --> n5(["End"])
      n4 --> n5
    end
    A["/del &lt;url&gt;"] --> B{"Parse URL"}
    B -- Supported Service --> C{"Get username"}
    B -- Unsupported Service --> D(["fa:fa-message Send failure message"])
    C -- Not Subscribed --> D
    C -- Subscribed --> E["fa:fa-message Send confirmation message"]
    E --> F{"Get<br>inline button<br>response"}
    F -- Confirmed --> s1
    s1 --> H(["fa:fa-message Change to success message"])
    F -- Cancelled --> I(["fa:fa-message Change to cancelled message"])
  ```
- List subscriptions
  ```mermaid
  flowchart LR
    subgraph s1["Get user subscription"]
      n1["Get the set named with the Telegram user ID"]
      n2(["Iterate over the set and return list value"])
      n1 --> n2
    end
    A["/list"] --> s1
    s1 -- Has at least 1 subscription --> C(["fa:fa-message Send subscription list"])
    s1 -- Has no subscription --> D(["fa:fa-message Send no subscription notice and help message"])
  ```
- Check live streams

  ```mermaid
  flowchart LR
    subgraph s1["Check running lives"]
      n1["Iterate through entries of hash named with platform:user_id:username"]
      n1 --> n2{"Check live state"}
      n2 -- Running --> n3(["End"])
      n2 -- Ended --> n4["fa:fa-message Send live ended<br>message to subscribers"]
      n4 --> n5["Set live ID of the subs hash and sub string key to empty"]
      n5 --> n6["Set chat ID of the sub string hash and Telegram user ID key to 0"]
      n6 --> n3
    end
    subgraph s2["Check new lives"]
      m1["Get live statuses from subscriptions vector"]
      m1 --> m2["Get keys of the hash named with the sub string as subscribers"]
      m2 --> m3["Notify subscribers for new live"]
      m3 --> m4["Set live ID of the subs hash and sub string key"]
      m4 --> m5(["Set message ID of the sub string hash and Telegram user ID key"])
    end
    A["Start"] --> B["Iterate through all entries of the platform from the 'subs' hash"]
    B --> C{"Has live ID<br>as value?"}
    C -- Yes --> s1
    C -- No --> D["Collect subscriptions into vector"]
    D --> s2
  ```
