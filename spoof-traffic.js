import http from 'k6/http';
import {sleep} from 'k6';

const slugs = ['api/users/1'];
const userAgents = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/114.0.0.0 Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15 Version/14.0 Mobile/15A372 Safari/604.1",
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
    "8.8.8.8",           // US - Google
    "1.1.1.1",           // AU - Cloudflare
    "5.255.255.70",      // RU - Yandex
    "213.180.204.3",     // RU - Yandex
    "31.13.71.36",       // IE - Facebook
    "66.220.144.0",      // US - Facebook
    "91.198.174.192",    // NL - Wikipedia
    "123.125.114.144",   // CN - Baidu
    "77.88.5.50",        // RU - Yandex DNS
    "210.140.92.183"     // JP - Twitter Japan
];

export const options = {
    insecureSkipTLSVerify: true,
};

function pickRandom(arr) {
    return arr[Math.floor(Math.random() * arr.length)];
}

export default function () {
    const slug = pickRandom(slugs);
    const url = `https://localhost:8443/${slug}`;

    const headers = {
        "User-Agent": pickRandom(userAgents),
        "Accept-Language": pickRandom(languages),
        "X-Forwarded-For": pickRandom(ips),
        "X-Real-IP": pickRandom(ips),
    };

    http.get(url, {headers});
    // throttle to 1 request per second per VU for more realistic traffic.
    sleep(1);
}
