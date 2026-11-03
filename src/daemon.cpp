#include <QCoreApplication>
#include <QDir>
#include <QLocalServer>
#include <QLocalSocket>
#include <QStandardPaths>
#include <QTimer>

#include <cstdio>
#include <utility>

#include "controller.h"
#include "orbstate.h"

namespace {

QByteArray snapshot(const OrbState &orb, double level)
{
    return QStringLiteral("{\"state\":\"%1\",\"busy\":%2,\"level\":%3}\n")
        .arg(orb.stateName(), orb.busy() ? QStringLiteral("true") : QStringLiteral("false"),
             QString::number(level, 'f', 3))
        .toUtf8();
}

} // namespace

// Headless half of the overlay: owns the microphone and the turn, and speaks
// JSON lines to whatever shell is drawing Newton. No window, no Qt Quick.
int runDaemon(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    app.setOrganizationName(QStringLiteral("Newton"));
    app.setApplicationName(QStringLiteral("Newton"));

    OrbState orb;
    Controller device(&orb, true);

    QLocalServer server;
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    const QString path = QDir(dir.isEmpty() ? QStringLiteral("/tmp") : dir)
                             .filePath(QStringLiteral("newton.sock"));
    QLocalServer::removeServer(path);
    if (!server.listen(path)) {
        fprintf(stderr, "newton: cannot listen on %s: %s\n", qPrintable(path),
                qPrintable(server.errorString()));
        return 1;
    }

    QList<QLocalSocket *> clients;
    auto send = [&](QLocalSocket *socket) {
        if (socket && socket->state() == QLocalSocket::ConnectedState)
            socket->write(snapshot(orb, device.level()));
    };

    QObject::connect(&server, &QLocalServer::newConnection, [&] {
        while (QLocalSocket *socket = server.nextPendingConnection()) {
            clients.append(socket);
            send(socket);
            QObject::connect(socket, &QLocalSocket::readyRead, socket, [&, socket] {
                while (socket->canReadLine()) {
                    const QByteArray line = socket->readLine().trimmed();
                    if (line.contains("talk"))
                        device.talk();
                    else if (line.contains("quit"))
                        QCoreApplication::quit();
                }
            });
            QObject::connect(socket, &QLocalSocket::disconnected, [&, socket] {
                clients.removeAll(socket);
                socket->deleteLater();
            });
        }
    });

    auto broadcast = [&] {
        const QByteArray line = snapshot(orb, device.level());
        for (QLocalSocket *socket : std::as_const(clients))
            if (socket->state() == QLocalSocket::ConnectedState)
                socket->write(line);
    };
    QObject::connect(&orb, &OrbState::stateChanged, broadcast);
    QObject::connect(&orb, &OrbState::busyChanged, broadcast);
    QObject::connect(&device, &Controller::levelChanged, broadcast);

    fprintf(stderr, "newton: daemon on %s (voice: %s)\n", qPrintable(path),
            qPrintable(device.voice()));
    return app.exec();
}
