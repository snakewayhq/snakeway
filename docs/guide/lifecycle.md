# Lifecycle


## Request/Response Phases

on_request -> before_proxy -> after_proxy -> on_response

## Phase Capabilities

| Phase        | Continue | Respond       | Error        |
| ------------ | -------- |---------------| ------------ |
| on_request   | proceed  | respond now   | respond 500  |
| before_proxy | proceed  | respond now   | respond 500  |
| after_proxy  | proceed  | override resp | mark error   |
| on_response  | proceed  | override resp | log + metric |
