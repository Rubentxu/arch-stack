# Sample Python module for class-diagram fixture.

class Base1:
    def __init__(self, value: int):
        self.value = value


class Base2:
    def __init__(self, label: str):
        self.label = label


class Derived(Base1, Base2):
    def __init__(self, value: int, label: str):
        Base1.__init__(self, value)
        Base2.__init__(self, label)
