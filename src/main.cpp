#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QDir>
#include <QEventLoop>
#include <QGuiApplication>
#include <QImage>
#include <QQmlApplicationEngine>
#include <QQmlError>
#include <QQuickWindow>
#include <QTimer>
#include <QVariant>

#include <QDebug>

#include "orbstate.h"

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    QCoreApplication::setOrganizationName(QStringLiteral("Newton"));
    QCoreApplication::setApplicationName(QStringLiteral("Newton"));

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Newton - local voice assistant orb"));
    parser.addHelpOption();
    QCommandLineOption checkOption(QStringLiteral("check"),
                                   QStringLiteral("Load the UI, walk every state, exit non-zero "
                                                  "on any QML error or warning."));
    parser.addOption(checkOption);
    QCommandLineOption shotsOption(QStringLiteral("shots"),
                                   QStringLiteral("Write one PNG per state into <dir> and exit."),
                                   QStringLiteral("dir"));
    parser.addOption(shotsOption);
    parser.process(app);
    const bool check = parser.isSet(checkOption);
    const QString shotsDir = parser.value(shotsOption);

    // Any QML error is a failure. Without this the app boots to a blank window
    // and nothing in CI notices.
    int problems = 0;
    QQmlApplicationEngine engine;
    QObject::connect(&engine, &QQmlApplicationEngine::warnings,
                     [&problems](const QList<QQmlError> &list) {
                         // fprintf as well as qWarning: the message handler is silent
                         // under some harnesses, and a blank window must not pass CI.
                         for (const QQmlError &e : list) {
                             qWarning().noquote() << e.toString();
                             fprintf(stderr, "QMLERR: %s\n", qPrintable(e.toString()));
                         }
                         problems += list.size();
                     });

    OrbState orb;
    engine.setInitialProperties({{QStringLiteral("orb"), QVariant::fromValue<QObject *>(&orb)}});
    engine.loadFromModule("Newton", "Main");

    if (engine.rootObjects().isEmpty()) {
        qCritical("Newton: root object failed to load");
        return 1;
    }

    if (!shotsDir.isEmpty()) {
        // A frame per state, so the ring can be looked at without a display.
        // Straight out of the same tree the app draws.
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().at(0));
        if (!window) {
            fprintf(stderr, "Newton: root object is not a window\n");
            return 1;
        }
        if (!QDir().mkpath(shotsDir)) {
            fprintf(stderr, "Newton: cannot create %s\n", qPrintable(shotsDir));
            return 1;
        }

        struct Shot
        {
            const char *name;
            OrbState::State state;
            bool busy;
        };
        const Shot shots[] = {
            {"booting", OrbState::State::Booting, false},
            {"standby", OrbState::State::StandBy, false},
            {"listening", OrbState::State::Listening, false},
            {"working", OrbState::State::Listening, true},
            {"speaking", OrbState::State::Speaking, false},
            {"error", OrbState::State::Error, false},
        };

        for (const Shot &shot : shots) {
            const bool moved = shot.state == OrbState::State::Error
                    ? (orb.fail(QStringLiteral("shot")), true)
                    : orb.state() == shot.state || orb.go(shot.state);
            if (!moved) {
                fprintf(stderr, "Newton: cannot reach %s from %s\n", shot.name,
                        qPrintable(orb.stateName()));
                return 1;
            }
            orb.setBusy(shot.busy);

            // Let the entry animation settle before the grab.
            QEventLoop settle;
            QTimer::singleShot(600, &settle, &QEventLoop::quit);
            settle.exec();

            const QString file = QDir(shotsDir).filePath(QStringLiteral("%1.png").arg(shot.name));
            if (!window->grabWindow().save(file)) {
                qCritical("Newton: cannot write %s", qPrintable(file));
                return 1;
            }
            printf("%s\n", qPrintable(file));
        }
        return problems == 0 ? 0 : 1;
    }

    if (check) {
        // Bindings for a state only evaluate once that state is rendered, so
        // walk them all instead of checking the first one.
        auto step = new QTimer(&app);
        step->setInterval(120);
        int tick = 0;
        QObject::connect(step, &QTimer::timeout, [&]() {
            switch (tick++) {
            case 0:
                orb.go(OrbState::State::StandBy);
                break;
            case 1:
                orb.go(OrbState::State::Listening);
                break;
            case 2:
                orb.setBusy(true);
                break;
            case 3:
                orb.go(OrbState::State::Speaking);
                break;
            case 4:
                orb.fail(QStringLiteral("check"));
                break;
            case 5:
                orb.go(OrbState::State::StandBy);
                break;
            default:
                step->stop();
                qInfo() << "Newton: qml check finished, problems =" << problems;
                QCoreApplication::exit(problems == 0 ? 0 : 1);
                return;
            }
        });
        step->start();
    }

    return app.exec();
}
