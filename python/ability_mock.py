from flask import Flask, request
import requests
import threading
import time

app = Flask(__name__)

class Ability:
    def __init__(self, name, attribute, property_name, http_url="127.0.0.1:8079"):
        self.name = name
        self.attribute = attribute
        self.property_name = property_name
        self.http_url = http_url
        self.headers = {'Content-Type': 'application/json'}
        self.status = 1 if "isonline" in property_name else 0
        self.regist()

    def regist(self):
        data = {
            "name": self.name,
            "http_url": self.http_url,
            "attributes": {
                self.attribute: {
                    "properties": {self.property_name: self.status}
                }
            }
        }
        res = requests.post(f"http://127.0.0.1:5000/register", headers=self.headers, json=data)

    def get(self):
        return self.status

    def change(self, status):
        self.status = status
        self.update_status()
        return self.status

    def update_status(self):
        data = {
            "name": self.name,
            "http_url": self.http_url,
            "attributes": {
                self.attribute: {
                    "properties": {self.property_name: self.status}
                }
            }
        }
        res = requests.put(f"http://127.0.0.1:5000/update", headers=self.headers, json=data)

class Camera(Ability):
    def __init__(self):
        super().__init__("sensor", "camera", "isonline")

class AlertCamera(Ability):
    def __init__(self):
        super().__init__("executor", "alert", "activate")

class Plc(Ability):
    def __init__(self):
        super().__init__("plc_sensor", "plc_1", "isonline")

class AlertPlc(Ability):
    def __init__(self):
        super().__init__("plc_executor", "alert_plc", "activate")

class Ability1(Ability):
    def __init__(self):
        super().__init__("ability1_sensor", "ability1", "isonline")

class Ability2(Ability):
    def __init__(self):
        super().__init__("ability2_sensor", "ability2", "isonline")

class Ability3(Ability):
    def __init__(self):
        super().__init__("ability3_sensor", "ability3", "isonline")

class Ability4(Ability):
    def __init__(self):
        super().__init__("ability4_sensor", "ability4", "isonline")

class Ability5(Ability):
    def __init__(self):
        super().__init__("ability5_sensor", "ability5", "isonline")

class Ability6(Ability):
    def __init__(self):
        super().__init__("ability6_sensor", "ability6", "isonline")

class Ability7(Ability):
    def __init__(self):
        super().__init__("ability7_sensor", "ability7", "isonline")

class Ability8(Ability):
    def __init__(self):
        super().__init__("ability8_sensor", "ability8", "isonline")

class Ability9(Ability):
    def __init__(self):
        super().__init__("ability9_sensor", "ability9", "isonline")

class Ability10(Ability):
    def __init__(self):
        super().__init__("ability10_sensor", "ability10", "isonline")

class AbilityAlert1(Ability):
    def __init__(self):
        super().__init__("ability1_executor", "alert_ability1", "activate")

class AbilityAlert2(Ability):
    def __init__(self):
        super().__init__("ability2_executor", "alert_ability2", "activate")

class AbilityAlert3(Ability):
    def __init__(self):
        super().__init__("ability3_executor", "alert_ability3", "activate")

class AbilityAlert4(Ability):
    def __init__(self):
        super().__init__("ability4_executor", "alert_ability4", "activate")

class AbilityAlert5(Ability):
    def __init__(self):
        super().__init__("ability5_executor", "alert_ability5", "activate")

class AbilityAlert6(Ability):
    def __init__(self):
        super().__init__("ability6_executor", "alert_ability6", "activate")

class AbilityAlert7(Ability):
    def __init__(self):
        super().__init__("ability7_executor", "alert_ability7", "activate")

class AbilityAlert8(Ability):
    def __init__(self):
        super().__init__("ability8_executor", "alert_ability8", "activate")

class AbilityAlert9(Ability):
    def __init__(self):
        super().__init__("ability9_executor", "alert_ability9", "activate")

class AbilityAlert10(Ability):
    def __init__(self):
        super().__init__("ability10_executor", "alert_ability10", "activate")
class AbilityAlert1(Ability):
    def __init__(self):
        super().__init__("ability1_executor", "alert_ability1", "activate")

class AbilityAlert2(Ability):
    def __init__(self):
        super().__init__("ability2_executor", "alert_ability2", "activate")

class AbilityAlert3(Ability):
    def __init__(self):
        super().__init__("ability3_executor", "alert_ability3", "activate")

class AbilityAlert4(Ability):
    def __init__(self):
        super().__init__("ability4_executor", "alert_ability4", "activate")

class AbilityAlert5(Ability):
    def __init__(self):
        super().__init__("ability5_executor", "alert_ability5", "activate")

class AbilityAlert6(Ability):
    def __init__(self):
        super().__init__("ability6_executor", "alert_ability6", "activate")

class AbilityAlert7(Ability):
    def __init__(self):
        super().__init__("ability7_executor", "alert_ability7", "activate")

class AbilityAlert8(Ability):
    def __init__(self):
        super().__init__("ability8_executor", "alert_ability8", "activate")

class AbilityAlert9(Ability):
    def __init__(self):
        super().__init__("ability9_executor", "alert_ability9", "activate")

class AbilityAlert10(Ability):
    def __init__(self):
        super().__init__("ability10_executor", "alert_ability10", "activate")
@app.route('/ability/get_status/<ability>', methods=['GET'])
def get_ability_status(ability):
    local_ability = globals().get(f"local_{ability}")
    if local_ability:
        return {"status": local_ability.get()}
    return {"error": "Ability not found"}, 404

@app.route('/ability/change_status/<ability>', methods=['POST'])
def change_ability_status(ability):
    local_ability = globals().get(f"local_{ability}")
    if local_ability:
        status = request.json["status"]
        if local_ability.change(status) == status:
            return "ok"
    return {"error": "Ability not found"}, 404

def toggle_ability_status(ability_name):
    while True:
        local_ability = globals().get(f"local_{ability_name}")
        if local_ability:
            new_status = 0 
            local_ability.change(new_status)
            data = {"status": new_status}
            try:
                response = requests.post(f"http://127.0.0.1:8079/ability/change_status/{ability_name}", json=data)
                print(f"Response: {response.status_code} - {response.text}")
            except requests.exceptions.RequestException as e:
                print(f"Request failed: {e}")
        time.sleep(10)

if __name__ == "__main__":
    local_camera = Camera()
    local_alertforcamera = AlertCamera()
    local_plc = Plc()
    local_alertforplc = AlertPlc()

    # 实例化新添加的10个能力
    local_ability1 = Ability1()
    local_ability2 = Ability2()
    local_ability3 = Ability3()
    local_ability4 = Ability4()
    local_ability5 = Ability5()
    local_ability6 = Ability6()
    local_ability7 = Ability7()
    local_ability8 = Ability8()
    local_ability9 = Ability9()
    local_ability10 = Ability10()

 # 实例化新添加的10个报警器能力
    local_alertforability1 = AbilityAlert1()
    local_alertforability2 = AbilityAlert2()
    local_alertforability3 = AbilityAlert3()
    local_alertforability4 = AbilityAlert4()
    local_alertforability5 = AbilityAlert5()
    local_alertforability6 = AbilityAlert6()
    local_alertforability7 = AbilityAlert7()
    local_alertforability8 = AbilityAlert8()
    local_alertforability9 = AbilityAlert9()
    local_alertforability10 = AbilityAlert10()
    # 启动线程来周期性地改变能力状态
    threading.Thread(target=toggle_ability_status, args=("camera",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("plc",), daemon=True).start()

    # 启动新能力的线程
    threading.Thread(target=toggle_ability_status, args=("ability1",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability2",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability3",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability4",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability5",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability6",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability7",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability8",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability9",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("ability10",), daemon=True).start()

    app.run(host='0.0.0.0', port=8079)
