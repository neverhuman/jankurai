import pickle

payload = b"\x80\x04N."
value = pickle.loads(payload)
