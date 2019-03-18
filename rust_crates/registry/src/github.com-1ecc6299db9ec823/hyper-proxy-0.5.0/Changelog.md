> Legend:
  - feat: A new feature
  - fix: A bug fix
  - docs: Documentation only changes
  - style: White-space, formatting, missing semi-colons, etc
  - refactor: A code change that neither fixes a bug nor adds a feature
  - perf: A code change that improves performance
  - test: Adding missing tests
  - chore: Changes to the build process or auxiliary tools/libraries/documentation

## 0.5.0
- feat: upgrade to hyper 0.12

## 0.4.1
- feat: make TLS support configurable

## 0.4.0
- feat: split Proxy into Proxy and ProxyConnector allowing to handle a list of proxies
- doc: add a set_proxy expression for http requests
- doc: fix some wrong comments
- perf: avoid one clone

## 0.3.0
- refactor: add a match_fn macro in tunnel
- fix: add missing '\' in connect message
- feat: do not use connect for pure http request. Else provide headers to update the primary request with.
- feat: have Custom intercept be an opaque struct using `Arc` to be Send + Sync + Clone

## 0.2.0
- feat: Add Intercept::None to never intercept any connection
- fix: Add Send + Sync constraints on Intercept::Custom function (breaking)
- feat: Make Intercept::matches function public
- feat: Add several function to get/modify internal states
