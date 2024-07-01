#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import paho.mqtt.client as client
import json
import time

HOST = "192.168.56.150"  # IP of edge
PORT = 1883

# pub 设备在线状态更新
TOPIC_STATE_UPDATE = "/state/update"
# pub twin属性获取
TOPIC_TWIN_GET = "/twin/get"
# sub twin属性的获取结果
TOPIC_TWIN_GET_RESULT = "/twin/get/result"
# pub 更新twin属性
TOPIC_TWIN_UPDATE = "/twin/update"
# sub 更新的结果，与发布的内容相同
TOPIC_UPDATE_RESULT = "/twin/update/result"
# sub 当twin/update的数据格式错误时，报错信息会发到这里
TOPIC_UPDATE_ERROR = "$hw/events/device//twin/update/result"


class Mapper(client.Client):

    '''
    这是一个单属性的KubeEdge Mapper实现.
    这个类继承自mqtt客户端(paho.mqtt.client.Client)
    '''

    def __init__(self, name, readonly, property):
        '''
        name: 设备实例名称
        readonly: 设备的属性是否只读
        property: 设备属性的名称
        '''
        # 设置mqtt客户端ID
        super(Mapper, self).__init__(name + "switch-mapper")
        # 设备属性的名字。代码只支持一个属性
        self.property = property
        # 设备实例的名称，作为所有mqtt消息的前缀
        self._topic_base = "$hw/events/device/" + name
        # 连接到mqtt broker
        self.connect(HOST, PORT)
        # 受到订阅消息时执行`_on_message`输出消息内容。
        # 所有需求单独处理的消息在下面注册回调
        self.on_message = _on_message
        # 只有可读写设备需要订阅以下消息
        if not readonly:
            # 得到查询到的twin信息，执行`_on_twin_get`
            self.message_callback_add(
                self._topic_base + TOPIC_TWIN_GET_RESULT, _on_twin_get)
            # 当twin信息被更新时，执行`_on_twin_update_result`
            self.message_callback_add(
                self._topic_base + TOPIC_UPDATE_RESULT, _on_twin_update_result)
            self.subscribe_suffix(TOPIC_TWIN_GET_RESULT)

        self.subscribe_suffix(TOPIC_UPDATE_RESULT)
        # 订阅格式错误的错误输出。这个地址包括了所有twin/update格式出错的消息，
        # 不只是这个设备的消息
        self.subscribe(TOPIC_UPDATE_ERROR)
        # 设置设备在线
        self._update_state("online")

    def subscribe_suffix(self, suffix):
        '''
        订阅主题为`$hw/events/device/设备名称/{suffix}`的消息
        '''
        print("SUB " + self._topic_base + suffix)
        self.subscribe(self._topic_base + suffix)

    def publish_suffix(self, suffix, payload):
        '''
        发布主题为`$hw/events/device/设备名称/{suffix}`,消息体为payload的消息
        '''
        print("PUB " + self._topic_base + suffix + " : " + payload)
        self.publish(self._topic_base + suffix, payload)

    def get_twin(self):
        '''
        获得twin状态,在TOPIC_TWIN_GET_RESULT消息上接收
        '''
        self.publish_suffix(TOPIC_TWIN_GET, "{}")

    def _update_state(self, state):
        payload = {
            'state': state
        }
        print("state:" + state)
        self.publish_suffix(TOPIC_STATE_UPDATE, json.dumps(payload))

    def update_twin(self, value):
        '''
        更新twin中属性的"actual"值为"value"
        '''
        payload = {
            'event_id': '',
            'timestamp': int(time.time()),
            'twin': {
                self.property: {
                    'actual': {
                        'value': value,
                    },
                    'metadata': {
                        'type': "Updated"
                    }
                }
            }
        }
        # print("actual:" + value)
        self.publish_suffix(TOPIC_TWIN_UPDATE, json.dumps(payload))

    def resolve(self, expected, actual):
        '''
        将设备的真实状态从"actual"更新到"expected"
        '''
        pass

    def get_actual(self):
        '''获得设备的真实状态'''
        pass


def _on_message(client, userdata, msg):
    print("GET " +
          msg.topic + " : " + str(msg.payload))


def _on_twin_update_result(client, userdata, msg):
    print("GET (_on_twin_update):" + msg.topic + " : " + str(msg.payload))
    result = json.loads(msg.payload)
    # device twin发送的../twin/update/result的数据和
    # 接收到的../twin/update数据保持一致.如果"expected"没有被更新
    # 则../twin/update/result不会包含"expected"的内容
    if 'expected' not in result['twin'][client.property]:
        return
    # expected被更新,重新获得当前twin的状态
    client.get_twin()


def _on_twin_get(client, userdata, msg):
    print("GET (_on_twin_get):" + msg.topic + " : " + str(msg.payload))
    result = json.loads(msg.payload)
    if 'expected' not in result['twin'][client.property]:
        return

    actual = None
    if 'actual' in result['twin'][client.property]:
        actual = result['twin'][client.property]['actual']['value']

    expected = result['twin'][client.property]['expected']['value']

    if (actual == None) or (actual != expected):
        client.resolve(expected, actual)
        # TODO: Mapper操作设备也可能是异步的
        # 这里Mapper并不一定可以同步的获得设备实际的状态
        client.update_twin(client.get_actual())
