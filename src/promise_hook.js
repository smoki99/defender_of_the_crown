export function crown_hook(promise, on_ok, on_err) {
  promise.then(on_ok).catch(on_err);
}
