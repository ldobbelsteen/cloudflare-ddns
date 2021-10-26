import time
import json
import sys
import socket
import yaml
import requests

API_URL = "https://api.cloudflare.com/client/v4"


def verify_token(token):
    try:
        header = {
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token
        }
        url = API_URL + "/user/tokens/verify"
        response = requests.request("GET", url, headers=header)
        if response.status_code != 200:
            raise Exception("invalid token")
        return header
    except:
        print("ERROR: Token is invalid")
        sys.exit()


def get_zone_id(zone, token):
    try:
        url = API_URL + "/zones?name=" + zone
        response = requests.request("GET", url, headers=token)
        return json.loads(response.text)["result"][0]["id"]
    except:
        print("ERROR: Zone could not be found")
        sys.exit()


def get_records(zone, name, token):
    try:
        url = API_URL + "/zones/" + zone + "/dns_records?type=A,AAAA&name=" + name
        response = requests.request("GET", url, headers=token)
        records = json.loads(response.text)["result"]
        a_record = next(
            (record for record in records if record["type"] == "A"), None)
        aaaa_record = next(
            (record for record in records if record["type"] == "AAAA"), None)
        return a_record, aaaa_record
    except:
        print("ERROR: Failed to fetch records")
        sys.exit()


def get_ip(version):
    try:
        if version == 4:
            ipv4 = requests.request(
                "GET", "http://ipv4.icanhazip.com").text.rstrip()
            socket.inet_pton(socket.AF_INET, ipv4)
            return ipv4
        if version == 6:
            ipv6 = requests.request(
                "GET", "http://ipv6.icanhazip.com").text.rstrip()
            socket.inet_pton(socket.AF_INET6, ipv6)
            return ipv6
    except:
        return None


def update_record(record, address, token):
    try:
        url = API_URL + "/zones/" + \
            record["zone_id"] + "/dns_records/" + record["id"]
        response = requests.request("PATCH", url, headers=token, json={
            "content": address
        })
        result = json.loads(response.text)["result"]
        record_type = record["type"]
        record_content = record["content"]
        result_content = result["content"]
        print(
            f"{record_type} record IP updated from {record_content} to {result_content}...")
        return result
    except:
        print("ERROR: Failed to update record")
        sys.exit()


def create_record(zone, properties, token):
    try:
        url = API_URL + "/zones/" + zone + "/dns_records"
        response = requests.request(
            "POST", url, headers=token, json=properties)
        result = json.loads(response.text)["result"]
        result_type = result["type"]
        result_content = result["content"]
        result_ttl = result["ttl"]
        result_proxied = "on" if result["proxied"] else "off"
        print(f"{result_type} record created with IP {result_content}, a TTL of {result_ttl} second(s) and proxying {result_proxied}...")
        return result
    except:
        print("ERROR: Failed to create record")
        sys.exit()


def remove_record(record, token):
    try:
        url = API_URL + "/zones/" + \
            record["zone_id"] + "/dns_records/" + record["id"]
        response = requests.request("DELETE", url, headers=token)
        result = json.loads(response.text)["result"]
        record_type = record["type"]
        print(f"{record_type} record has been removed...")
        return result
    except:
        print("ERROR: Failed to remove record")
        sys.exit()


def update_routine(config):
    a_record, aaaa_record = get_records(
        config["zone"], config["record"], config["token"])
    if a_record is not None or config["morphing"]:
        ipv4 = None
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
        ipv6 = None
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
            aaaa_record = create_record(
                config["zone"], details, config["token"])
        elif aaaa_record is not None and ipv6 is None and config["morphing"]:
            aaaa_record = remove_record(aaaa_record, config["token"])


if __name__ == "__main__":
    conf = yaml.safe_load(open(sys.argv[1], encoding="UTF-8"))
    conf["record"] = conf["zone"] if conf["record"] == "@" else conf["record"] + \
        "." + conf["zone"]
    conf['token'] = verify_token(conf['token'])
    conf['zone'] = get_zone_id(conf['zone'], conf['token'])
    if conf["timer"] == 0:
        update_routine(conf)
    else:
        while True:
            update_routine(conf)
            time.sleep(conf['timer'])
