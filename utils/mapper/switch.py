#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from mapper import Mapper
import time

class Switch(Mapper):
    def __init__(self, name):
        self._status = '0'
        super(Switch, self).__init__(name, False, "status")
#在初始化时，Switch实例的状态（_status）被设置为'0'，代表关闭状态。
#super(Switch, self).__init__(name, False, "status")调用基类Mapper的构造方法。这里传递了设备的名称name、一个布尔值False（可能表示是否活动或是否可用），以及字符串"status"（可能指示这是一个状态设备）。
    def resolve(self, expected, actual):#法用于处理外部设置的期望状态。这里的expected是外部期望的状态值，而actual参数在方法体中未被使用。
        if expected == '0':
            print("\x1B[31mTurn OFF\x1B[39;49m in " + str(time.time_ns()) + " ns")
            self._status = '0'
        elif expected == '1':
            print("\x1B[32mTrun ON\x1B[39;49m in " + str(time.time_ns()) + " ns")
            self._status = '1'
        else:
            print("Invalid value:" + expected)

    def get_actual(self):#get_actual方法返回开关的当前状态（_status）。
        return self._status

if __name__ == '__main__':
    sw = Switch("switch")
    sw.loop_forever()#在脚本的主区块中，创建了一个Switch实例，并假设Switch（或其基类Mapper）有一个名为loop_forever的方法，用于启动设备的主循环或监听状态改变的循环。
