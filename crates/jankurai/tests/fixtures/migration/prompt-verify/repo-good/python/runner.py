from openai import OpenAI


class Runner(BaseRunner):
    def run(self):
        client = OpenAI()
        return client.responses.create(model="gpt-4.1", input="hello")
