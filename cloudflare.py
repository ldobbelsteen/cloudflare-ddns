import requests
import socket
import time
import json
import yaml
import sys

api_url = "https://api.cloudflare.com/client/v4"

def verify_token(token):
    header = {
        "Content-Type": "application/json",
        "Authorization": "Bearer " + token
    }
    url = api_url + "/user/tokens/verify"
    response = requests.request("GET", url, headers=header)
    if response.status_code != 200:
        print("ERROR: Token is invalid")
        exit()
    else:
        return header

def get_zone_id(zone, token):
    try:
        url = api_url + "/zones?name=" + zone
        response = requests.request("GET", url, headers=token)
        return json.loads(response.text)["result"][0]["id"]
    except:
        print("ERROR: Zone could not be found")
        exit()

def get_records(zone, name, token):
    url = api_url + "/zones/" + zone + "/dns_records?type=A,AAAA&name=" + name
    response = requests.request("GET", url, headers=token)
    records = json.loads(response.text)["result"]
    a_record = next((record for record in records if record["type"] == "A"), None)
    aaaa_record = next((record for record in records if record["type"] == "AAAA"), None)
    return a_record, aaaa_record

def get_ip(version):
    try:
        if version == 4:
            ipv4 = requests.request("GET", "http://ipv4.icanhazip.com").text.rstrip()
            socket.inet_pton(socket.AF_INET, ipv4)
            return ipv4
        if version == 6:
            ipv6 = requests.request("GET", "http://ipv6.icanhazip.com").text.rstrip()
            socket.inet_pton(socket.AF_INET6, ipv6)
            return ipv6
    except:
        return None

def update_record(record, ip, token):
    url = api_url + "/zones/" + record["zone_id"] + "/dns_records/" + record["id"]
    response = requests.request("PATCH", url, headers=token, json={
        "content": ip
    })
    result = json.loads(response.text)["result"]
    print("{} record IP updated from {} to {}...".format(record["type"], record["content"], result["content"]))
    return result

def create_record(zone, properties, token):
    url = api_url + "/zones/" + zone + "/dns_records"
    response = requests.request("POST", url, headers=token, json=properties)
    result = json.loads(response.text)["result"]
    print("{} record created with IP {}, a TTL of {} second(s) and proxying {}...".format(result["type"], result["content"], result["ttl"], "on" if result["proxied"] else "off"))
    return result

def remove_record(record, token):
    url = api_url + "/zones/" + record["zone_id"] + "/dns_records/" + record["id"]
    response = requests.request("DELETE", url, headers=token)
    result = json.loads(response.text)["result"]
    print("{} record has been removed...".format(record["type"]))
    return result

def update_routine(config):
    a_record, aaaa_record = get_records(config["zone"], config["record"], config["token"])
    if a_record is not None or config["morphing"]:
        if not config["disable_ipv4"]:
            ipv4 = get_ip(4)
        if a_record is not None and ipv4 is not None and ipv4 != a_record["content"]:
            a_record = update_record(a_record, ipv4, config["token"])
        elif a_record is None and ipv4 is not None and config["morphing"]:
            details = {
                "type": "A",
                "name": config["record"],
                "content": ipv4,
                "proxied": True if aaaa_record is None else aaaa_record["proxied"],
                "ttl": 1 if aaaa_record is None else aaaa_record["ttl"]
            }
            a_record = create_record(config["zone"], details, config["token"])
        elif a_record is not None and ipv4 is None and config["morphing"]:
            a_record = remove_record(a_record, config["token"])
    if aaaa_record is not None or config["morphing"]:
        if not config["disable_ipv6"]:
            ipv6 = get_ip(6)
        if aaaa_record is not None and ipv6 is not None and ipv6 != aaaa_record["content"]:
            aaaa_record = update_record(aaaa_record, ipv6, config["token"])
        elif aaaa_record is None and ipv6 is not None and config["morphing"]:
            details = {
                "type": "AAAA",
                "name": config["record"],
                "content": ipv6,
                "proxied": True if a_record is None else a_record["proxied"],
                "ttl": 1 if a_record is None else a_record["ttl"]
            }
            aaaa_record = create_record(config["zone"], details, config["token"])
        elif aaaa_record is not None and ipv6 is None and config["morphing"]:
            aaaa_record = remove_record(aaaa_record, config["token"])

def main():
    config = yaml.safe_load(open(sys.argv[1]))
    config["record"] = config["zone"] if config["record"] == "@" else config["record"] + "." + config["zone"]
    config['token'] = verify_token(config['token'])
    config['zone'] = get_zone_id(config['zone'], config['token'])
    if config["timer"] == 0:
        update_routine(config)
    else:
        while True:
            update_routine(config)
            time.sleep(config['timer'])

if __name__ == "__main__":
    main()
