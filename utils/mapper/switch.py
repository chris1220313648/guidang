#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from mapper import Mapper
import time

class Switch(Mapper):
    def __init__(self, name):
        self._status = '0'
        super(Switch, self).__init__(name, False, "status")

    def resolve(self, expected, actual):
        if expected == '0':
            print("\x1B[31mTurn OFF\x1B[39;49m in " + str(time.time_ns()) + " ns")
            self._status = '0'
        elif expected == '1':
            print("\x1B[32mTrun ON\x1B[39;49m in " + str(time.time_ns()) + " ns")
            self._status = '1'
        else:
            print("Invalid value:" + expected)

    def get_actual(self):
        return self._status

if __name__ == '__main__':
    sw = Switch("switch")
    sw.loop_forever()
