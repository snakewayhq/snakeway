import http from "k6/http";
import {check} from "k6";
import crypto from "k6/crypto";
import encoding from "k6/encoding";

/*
JWT auth WASM device load test
------------------------------

Drives the JWT auth device hot path. Every request carries one valid HS256
bearer token, so on_request runs the full validation path (config read, base64
decode, HMAC verify, claim checks) on each call.

Pair this with a snakeway build that has the `hotpath` (and optionally
`hotpath-alloc`) feature enabled. Stop the server with SIGINT after the run to
flush the hotpath report. See `just profile-wasm-timing` / `just profile-wasm-alloc`.

The token is signed to match config/device.d/wasm.hcl:
  secret   = base64("secret")  -> HMAC key "secret"
  issuer   = https://auth.example.com
  audience = https://api.example.com
  sub      = user-1

Env overrides:
  TARGET_URL  request URL (default http://snakeway.test:8080/api/users/1)
  JWT_SECRET  HMAC key (default "secret")
*/

const SECRET = __ENV.JWT_SECRET || "secret";
const TARGET_URL = __ENV.TARGET_URL || "http://snakeway.test:8080/api/users/1";

// rawurl = URL-safe base64 with no padding (JWT segment encoding).
function b64url(input) {
    return encoding.b64encode(input, "rawurl");
}

function mintToken() {
    const header = JSON.stringify({alg: "HS256", typ: "JWT"});
    const payload = JSON.stringify({
        sub: "user-1",
        iss: "https://auth.example.com",
        aud: "https://api.example.com",
        exp: 4102444800, // 2100-01-01; effectively non-expiring for tests
    });
    const signingInput = `${b64url(header)}.${b64url(payload)}`;
    const signature = crypto.hmac("sha256", SECRET, signingInput, "base64rawurl");
    return `${signingInput}.${signature}`;
}

// Minted once at init so token creation stays off the per-request hot path.
const TOKEN = mintToken();

export const options = {
    insecureSkipTLSVerify: true,
};

export default function () {
    const res = http.get(TARGET_URL, {
        headers: {Authorization: `Bearer ${TOKEN}`},
    });

    check(res, {
        "status is 200": (r) => r.status === 200,
        "not auth-rejected": (r) => r.status !== 401 && r.status !== 403,
    });
}
