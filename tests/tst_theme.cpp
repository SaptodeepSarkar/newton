#include <QtTest>
#include <QQmlComponent>
#include <QQmlEngine>

// Loads the real Theme singleton and pins every colour and stroke ratio to the
// measured renders in docs/UI/Visual Spec.md, so a silent hex drift fails CI.
class ThemeTest : public QObject
{
    Q_OBJECT

    QQmlEngine m_engine;
    QObject *m_theme = nullptr;

    QVariant call(const char *fn, const QVariantList &args)
    {
        QVariant ret;
        // QML functions take QVariant parameters, so arity must match exactly.
        if (args.size() == 3)
            QMetaObject::invokeMethod(m_theme, fn, Q_RETURN_ARG(QVariant, ret),
                                      Q_ARG(QVariant, args.value(0)),
                                      Q_ARG(QVariant, args.value(1)),
                                      Q_ARG(QVariant, args.value(2)));
        else if (args.size() == 2)
            QMetaObject::invokeMethod(m_theme, fn, Q_RETURN_ARG(QVariant, ret),
                                      Q_ARG(QVariant, args.value(0)),
                                      Q_ARG(QVariant, args.value(1)));
        else
            QMetaObject::invokeMethod(m_theme, fn, Q_RETURN_ARG(QVariant, ret),
                                      Q_ARG(QVariant, args.value(0)));
        // QML functions are declared to return QVariant, so unwrap the box.
        qDebug() << "ret" << ret.typeName() << ret; if (ret.typeId() == QMetaType::QVariant) { ret = ret.value<QVariant>(); qDebug() << "inner" << ret.typeName() << ret; }
        return ret;
    }

private slots:
    void initTestCase()
    {
        // Path is relative to this file, not the build tree: build/src has a
        // generated qmldir that would make the engine want the plugin.
        const QString path = QFileInfo(QStringLiteral(__FILE__)).absolutePath() + "/../src/Theme.qml";
        QVERIFY2(QFileInfo::exists(path), qPrintable(path));
        QQmlComponent component(&m_engine, QUrl::fromLocalFile(path));
        QVERIFY2(component.isReady(), qPrintable(component.errorString()));
        m_theme = component.create();
        QVERIFY2(m_theme, qPrintable(component.errorString()));
    }

    void colours()
    {
        const QVector<QPair<const char *, const char *>> expected = {
            {"surface", "#1b1917"},
            {"booting", "#fbfaf8"},
            {"standby", "#44f7f4"},
            {"listening", "#f1f59c"},
            {"speaking", "#776eec"},
            {"speakingLoud", "#9893f5"},
            {"error", "#e5484d"},
        };
        for (const auto &e : expected)
            QCOMPARE(m_theme->property(e.first).value<QColor>().name(QColor::HexRgb),
                     QString::fromLatin1(e.second));
    }

    void colorFor()
    {
        QCOMPARE(call("colorFor", {QStringLiteral("speaking"), false}).value<QColor>().name(),
                 QStringLiteral("#776eec"));
        QCOMPARE(call("colorFor", {QStringLiteral("speaking"), true}).value<QColor>().name(),
                 QStringLiteral("#9893f5"));
        QCOMPARE(call("colorFor", {QStringLiteral("standby"), true}).value<QColor>().name(),
                 QStringLiteral("#44f7f4"));
    }

    void strokes()
    {
        const QVector<QPair<QVariantList, double>> expected = {
            {{"booting", false, false}, 0.011},
            {{"standby", false, false}, 0.22},
            {{"listening", false, false}, 0.18},
            {{"listening", true, false}, 0.12},
            {{"speaking", false, false}, 0.12},
            {{"speaking", false, true}, 0.25},
            {{"error", false, false}, 0.18},
        };
        for (const auto &e : expected)
            QCOMPARE(call("strokeFor", e.first).toDouble(), e.second);
    }
};

QTEST_GUILESS_MAIN(ThemeTest)
#include "tst_theme.moc"
