#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from mapper import Mapper
import time


class Temp(Mapper):
    def __init__(self, name):
        self.status = '0'
        super(Temp, self).__init__(name, True, "temperature")

    def get_actual(self):
        return self.status


if __name__ == '__main__':
    temp = Temp("dht11")
    temp.loop_start()
    while True:
        val = input()
        _int = float(val)
        print("update to " + val )
        temp.update_twin(val)#update_twin方法更新设备的状态（这个方法可能也是基类Mapper提供的）
        temp.status = val
