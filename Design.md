# Design

## Commands

- `/sub <url>`: subscribe to the live stream from the specified URL
- `/del <url>`: delete the subscription to the live stream of the specified URL
- `/list`: list all subscriptions of the current user
- `/platform`: list all supported platforms

## Workflows

- Add subscription
  ```mermaid
  flowchart LR
    subgraph s1["Add subscription"]
      n1["Add Telegram user ID to the set<br>named with Platform:user_id:username"] --> n2["Add the pair to the set<br>named with the Telegram user ID"]
      n2 --> n3(["Add the pair to the set 'subs'"])
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
      n1["Remove the pair from the set<br>named with the Telegram user ID"]
      n1 --> n2{"Does the set<br>Platform:user_id:username<br>only have 1<br>Telegram user ID?"}
      n2 -- Yes --> n3["Remove the pair from the 'subs' set"]
      n2 -- No --> n4(["Remove Telegram user ID from the set<br>named with Platform:user_id:username"])
      n3 --> n4
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
