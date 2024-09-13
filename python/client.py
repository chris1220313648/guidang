from flask import Flask, request
import requests
if __name__ == "__main__":
    headers = {'Content-Type': 'application/json'}
    while True:
        status = input("1 for on and 0 for off:")
        if status == 1:
            data = {"status": True}
        else:
            data = {"status": False}
        res = requests.post("http://127.0.0.1:8079/camera/change_status", headers = headers, json = data)
