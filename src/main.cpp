#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlError>
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
    parser.process(app);
    const bool check = parser.isSet(checkOption);

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
