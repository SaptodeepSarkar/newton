#include <QCoreApplication>
#include <QDir>
#include <QLocalServer>
#include <QLocalSocket>
#include <QStandardPaths>
#include <QTimer>

#include <cstdio>
#include <utility>

#include <QFileInfo>

#include "audio/Mic.h"
#include "audio/WakeModel.h"
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

// Enrolment: the owner says the wake word a few times and it is saved as a
// template. No training run, no downloaded weights, no speech model.
int runEnroll(int argc, char **argv, int count)
{
    QCoreApplication app(argc, argv);
    app.setOrganizationName(QStringLiteral("Newton"));
    app.setApplicationName(QStringLiteral("Newton"));

    Mic mic;
    WakeModel model;
    int got = 0;

    auto prompt = [&] { fprintf(stderr, "say the wake word (%d to go)\n", count - got); };

    QObject::connect(&mic, &Mic::speechStarted, [] { fprintf(stderr, "  heard you...\n"); });
    QObject::connect(&mic, &Mic::speechEnded, [&] {
        const QVector<qint16> pcm = mic.utterance();
        if (!model.enroll(pcm)) {
            fprintf(stderr, "  too short - say it again\n");
            return;
        }
        ++got;
        fprintf(stderr, "  sample %d: %d samples, matches itself at %.2f\n", got, int(pcm.size()),
                model.score(pcm));
        if (got < count) {
            prompt();
            return;
        }

        const QString path = Controller::wakeModelPath();
        QDir().mkpath(QFileInfo(path).absolutePath());
        QString error;
        if (!model.save(path, &error)) {
            fprintf(stderr, "cannot write %s: %s\n", qPrintable(path), qPrintable(error));
            app.exit(1);
            return;
        }
        const double threshold = model.calibrate();
        if (!model.save(path, &error)) {
            fprintf(stderr, "cannot re-save %s: %s\n", qPrintable(path), qPrintable(error));
            app.exit(1);
            return;
        }
        fprintf(stderr, "wake model saved: %s (%d templates, threshold %.2f)\n",
                qPrintable(path), model.templates(), threshold);
        app.quit();
    });

    mic.start();
    if (!mic.isRunning()) {
        fprintf(stderr, "newton: no microphone for enrolment\n");
        return 1;
    }
    prompt();
    return app.exec();
}

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
