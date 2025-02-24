from . import sm


def main():
    sm.run()


if __name__ == "__main__":
    try:
        main()
    except (EOFError, KeyboardInterrupt):
        exit()
