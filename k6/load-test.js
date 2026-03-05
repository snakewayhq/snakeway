import http from "k6/http";
import {check, sleep} from "k6";
import {Trend} from "k6/metrics";

/*
Reverse proxy load test
-----------------------

Designed to test:

* TLS termination
* connection reuse
* header parsing (UA/IP)
* upstream proxying
*/

export const options = {
    insecureSkipTLSVerify: true,

    scenarios: {
        ramp: {
            executor: "ramping-vus",
            startVUs: 0,
            stages: [
                {duration: "30s", target: 50},   // warmup
                {duration: "60s", target: 200},  // steady load
                {duration: "30s", target: 500},  // burst spike
                {duration: "30s", target: 0},    // cooldown
            ],
        },
    },
};

const latency = new Trend("proxy_latency");

const baseUrl = "https://snakeway.test:8443";

const slugs = [
    "api/users/1",
];

const userAgents = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/114.0.0.0 Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15 Version/14.0 Mobile Safari/604.1",
    "Mozilla/5.0 (Linux; Android 11; SM-G981B) AppleWebKit/537.36 Chrome/103.0.5060.71 Mobile Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/112.0.5615.49 Safari/537.36",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:98.0) Gecko/20100101 Firefox/98.0",
    "curl/7.68.0",
    "PostmanRuntime/7.29.2",
    "Googlebot/2.1 (+http://www.google.com/bot.html)",
    "Bingbot/2.0 (+http://www.bing.com/bingbot.htm)",
    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)"
];

const languages = ["en-US", "fr-FR", "de-DE"];

const ips = [
    "8.8.8.8",
    "1.1.1.1",
    "5.255.255.70",
    "213.180.204.3",
    "31.13.71.36",
    "66.220.144.0",
    "91.198.174.192",
    "123.125.114.144",
    "77.88.5.50",
    "210.140.92.183"
];

function pickRandom(arr) {
    return arr[Math.floor(Math.random() * arr.length)];
}

export default function () {
    const slug = pickRandom(slugs);

    const url = `${baseUrl}/${slug}`;

    const headers = {
        "User-Agent": pickRandom(userAgents),
        "Accept-Language": pickRandom(languages),
        "X-Forwarded-For": pickRandom(ips),
    };

    const res = http.get(url, {headers});

    latency.add(res.timings.duration);

    check(res, {
        "status is 200": (r) => r.status === 200,
        "latency < 200ms": (r) => r.timings.duration < 200,
    });

// small jitter so requests aren't perfectly synchronized
    sleep(Math.random() * 0.2);
}
