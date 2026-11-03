#include <QQmlComponent>
#include <QQmlEngine>
#include <QSignalSpy>
#include <QtTest>

namespace {
QStringList warnings;

void collect(QtMsgType type, const QMessageLogContext &, const QString &msg)
{
    if (type == QtWarningMsg || type == QtCriticalMsg)
        warnings.append(msg);
}
}

class Screens : public QObject
{
    Q_OBJECT

    QQmlEngine engine;

    QObject *load(const QString &type)
    {
        QQmlComponent component(&engine);
        component.loadFromModule("Newton", type);
        if (!component.isReady()) {
            qWarning("%s", qPrintable(component.errorString()));
            return nullptr;
        }
        return component.create();
    }

private slots:
    // No window means no real press, so drive the handler the MouseArea calls.
    bool click(QObject *target, const char *handler)
    {
        return QMetaObject::invokeMethod(target, handler, Qt::DirectConnection);
    }

    void init()
    {
        warnings.clear();
        qInstallMessageHandler(collect);
    }

    void cleanup() { qInstallMessageHandler(nullptr); }

    void firstRun()
    {
        QScopedPointer<QObject> screen(load("FirstRun"));
        QVERIFY(screen);
        QCOMPARE(screen->property("finished").toBool(), false);

        QSignalSpy done(screen.data(), SIGNAL(done()));
        auto *button = screen->findChild<QObject *>("continueButton");
        QVERIFY(button);
        QVERIFY(click(screen.data(), "activate"));

        QCOMPARE(screen->property("finished").toBool(), true);
        QCOMPARE(done.count(), 1);
        QVERIFY2(warnings.isEmpty(), qPrintable(warnings.join('\n')));
    }

    void settings()
    {
        QScopedPointer<QObject> screen(load("Settings"));
        QVERIFY(screen);
        QCOMPARE(screen->property("muted").toBool(), false);

        QSignalSpy muteChanged(screen.data(), SIGNAL(muteChanged(bool)));
        auto *toggle = screen->findChild<QObject *>("muteToggle");
        QVERIFY(toggle);
        QVERIFY(click(screen.data(), "toggleMute"));
        QCOMPARE(screen->property("muted").toBool(), true);
        QCOMPARE(muteChanged.count(), 1);
        QCOMPARE(muteChanged.takeFirst().at(0).toBool(), true);

        QVERIFY(click(screen.data(), "toggleMute"));
        QCOMPARE(screen->property("muted").toBool(), false);
        QCOMPARE(muteChanged.count(), 1);
        QCOMPARE(muteChanged.takeFirst().at(0).toBool(), false);

        QSignalSpy forgetAll(screen.data(), SIGNAL(forgetAll()));
        auto *forget = screen->findChild<QObject *>("forgetButton");
        QVERIFY(forget);
        QVERIFY(click(screen.data(), "forget"));
        QCOMPARE(forgetAll.count(), 0);
        QVERIFY(click(screen.data(), "forget"));
        QCOMPARE(forgetAll.count(), 1);
        QVERIFY2(warnings.isEmpty(), qPrintable(warnings.join('\n')));
    }
};

QTEST_MAIN(Screens)
#include "tst_screens.moc"
