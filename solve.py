#!/usr/bin/env python3
from Crypto.Cipher import AES
from binascii import a2b_hex, b2a_hex
import random

# 已知的固定key和iv
old_key = b'73E5602B54FE63A5'
old_iv = b'B435AE462FBAA662'

def add_to_16(text):
    if len(text.encode('utf-8')) % 16:
        add = 16 - (len(text.encode('utf-8')) % 16)
    else:
        add = 0
    text = text + ('\0' * add)
    return text.encode('utf-8')

def decrypt(ciphertext, key, iv):
    mode = AES.MODE_CBC
    cryptos = AES.new(key, mode, iv)
    plaintext = cryptos.decrypt(ciphertext)
    return plaintext

# 读取提供的密文
ciphertexts = []
with open('msg.txt', 'r') as f:
    for line in f:
        line = line.strip()
        if line:
            ciphertexts.append(a2b_hex(line))

print(f"读取了 {len(ciphertexts)} 个密文")

# 解密前几个密文看看
for i in range(3):
    plaintext = decrypt(ciphertexts[i], old_key, old_iv)
    print(f"密文 {i}: {b2a_hex(ciphertexts[i])}")
    print(f"明文 {i}: {plaintext}")
    print(f"明文hex {i}: {b2a_hex(plaintext)}")
    print()

