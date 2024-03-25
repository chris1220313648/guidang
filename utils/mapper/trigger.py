#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import paho.mqtt.client as client#核心客户端库，提供了与MQTT服务器交互的功能。
import json
import time

HOST = "127.0.0.1"  # IP of edge
PORT = 1883

TOPIC_UPDATE = "$hw/events/device/dht11/twin/update/result"

cli = client.Client("fake-trigger")#建了一个新的MQTT客户端实例，"fake-trigger"是该客户端的标识符。

cli.connect(HOST, PORT)
cli.publish(TOPIC_UPDATE, payload="{}")
cli.loop_start()#开始MQTT客户端的网络循环。这个循环负责处理网络事件，包括重新连接和重新发布未确认的消息。
time.sleep(1)
exit()