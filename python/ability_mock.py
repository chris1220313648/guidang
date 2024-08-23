from flask import Flask, request
import requests
register_url=""
app = Flask(__name__)


class camera:
    def __init__(self):
        self.isonline = True
        self.headers = {'Content-Type': 'application/x-www-form-urlencoded'}
        self.regist()

    def regist(self):
        data = '{"name": "sensor", "attributes": ["camera": {"properties": ["isonline"], "address": "127.0.0.1:8079"}]}'
        res = requests.request("POST","http://127.0.0.1:5000/register", headers = self.headers, data = data)
        print(res.text)

    def get(self):
        return self.isonline

    def change(self, status):
        self.isonline = status
        return self.isonline


class alert:
    def __init__(self):
        self.activate = False
        self.headers = {'Content-Type': 'application/x-www-form-urlencoded'}
        self.regist()

    def regist(self):
        data = '{"name": "executor", "attributes": ["alert": {"properties": ["activate"], "address": "127.0.0.1:8079"}]}'
        res = requests.request("POST","http://127.0.0.1:5000/register", headers = self.headers, data = data)
        print(res.text)

    def get(self):
        return self.activate

    def change(self, status):
        self.activate = status
        return self.activate


@app.route('/camera/get_status', methods=['GET'])
def get_camera_status():
    return {"status": local_camera.get()}


@app.route('/camera/change_status', methods=['POST'])
def change_camera_status():
    if local_camera.change(request.json["status"]) == request.json["status"]:
        return "ok"
# {"status": True}


@app.route('/alert/get_status', methods=['GET'])
def get_alert_status():
    return {"status": local_alert.get()}


@app.route('/alert/change_status', methods=['POST'])
def change_alert_status():
    if local_camera.change(request.json["status"]) == request.json["status"]:
        return "ok"
# {"status": True}


if __name__ == "__main__":
    local_camera = camera()
    local_alert = alert()
    app.run(host='0.0.0.0', port=8079)
