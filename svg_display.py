import sys
from PyQt5 import QtCore, QtWidgets, QtSvg


def display(path):
    app = QtWidgets.QApplication(sys.argv)
    window = QtWidgets.QMainWindow()
    widget = QtSvg.QSvgWidget(path, window)
    widget.renderer().setAspectRatioMode(QtCore.Qt.AspectRatioMode(1))
    window.setCentralWidget(widget)
    window.show()
    sys.exit(app.exec_())


if __name__ == '__main__':
    display('./test_render.svg')
