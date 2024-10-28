# Design

## Commands

- `/sub <url>`: subscribe to the live stream from the specified URL
- `/del <url>`: delete the subscription to the live stream of the specified URL
- `/list`: list all subscriptions of the current user

## Workflows

- Add subscription
  ```mermaid
  flowchart LR
    subgraph s1["Add subscription"]
      n1{"Find subscription user ID in subs hashtable"}
      n1 -- Found --> n2["Add Telegram user ID to returned list"]
      n1 -- Not Found --> n3["Add subscription user ID as the key to the subs hashtable"]
      n3 --> n2
      n2 --> n4{"Find Telegram user ID in users hashtable"}
      n4 -- Found --> n5(["Add subscription user ID to the returned list"])
      n4 -- Not Found --> n6["Add Telegram user ID as the key to the users hashtable"]
      n6 --> n5
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
      n1["Find subscription user ID in subs hashtable"]
      n2{"Is the Telegram user ID the only element in the returned list?"}
      n1 --> n2
      n2 -- Yes --> n3["Remove the entire key from the subs hashtable"]
      n2 -- No --> n4["Remove Telegram user ID from returned list"]
      n5["Find Telegram user ID in users hashtable"]
      n3 --> n5
      n4 --> n5
      n5 --> n6{"Is the subscription user ID the only element in the returned list?"}
      n6 -- Yes --> n7["Remove the entire key from the users hashtable"]
      n6 -- No --> n8["Remove subscription user ID from returned list"]
      n9(["Finish"])
      n7 --> n9
      n8 --> n9
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
      n1["Find Telegram user ID in users hashtable"]
      n2(["Return list value"])
      n1 --> n2
    end
    A["/list"] --> s1
    s1 -- Has at least 1 subscription --> C(["fa:fa-message Send subscription list"])
    s1 -- Has no subscription --> D(["fa:fa-message Send no subscription notice and help message"])
  ```
