#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import paho.mqtt.client as client
import json
import time

HOST = "127.0.0.1"  # IP of edge
PORT = 1883

TOPIC_UPDATE = "$hw/events/device/dht11/twin/update/result"

cli = client.Client("fake-trigger")

cli.connect(HOST, PORT)
cli.publish(TOPIC_UPDATE, payload="{}")
cli.loop_start()
time.sleep(1)
exit()