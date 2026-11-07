#include "WakeModel.h"

#include <QFile>
#include <QTextStream>
#include <QtMath>

#include <algorithm>
#include <cmath>

namespace {
constexpr int Rate = 16000;
constexpr int Window = 400;   // 25 ms
constexpr int Hop = 160;      // 10 ms
constexpr int Bands = 20;
constexpr int MinFrames = 12; // shorter than a spoken word

// Mel scale, so the bands are spaced the way hearing is, not the way a ruler is.
double melToHz(double mel)
{
    return 700.0 * (std::pow(10.0, mel / 2595.0) - 1.0);
}

double hzToMel(double hz)
{
    return 2595.0 * std::log10(1.0 + hz / 700.0);
}

QVector<double> bandCenters()
{
    const double low = hzToMel(120.0);
    const double high = hzToMel(7000.0);
    QVector<double> centers;
    centers.reserve(Bands);
    for (int i = 0; i < Bands; ++i)
        centers.append(melToHz(low + (high - low) * (i + 0.5) / Bands));
    return centers;
}

// Goertzel: the energy at one frequency, without an FFT or a dependency.
double goertzel(const qint16 *samples, int count, double hz)
{
    const double w = 2.0 * M_PI * hz / Rate;
    const double coeff = 2.0 * std::cos(w);
    double s0 = 0.0;
    double s1 = 0.0;
    double s2 = 0.0;
    for (int i = 0; i < count; ++i) {
        const double window = 0.54 - 0.46 * std::cos(2.0 * M_PI * i / (count - 1));
        s0 = samples[i] / 32768.0 * window + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    return s1 * s1 + s2 * s2 - coeff * s1 * s2;
}
} // namespace

QVector<QVector<float>> WakeModel::features(const QVector<qint16> &pcm)
{
    QVector<QVector<float>> out;
    if (pcm.size() < Window)
        return out;

    static const QVector<double> centers = bandCenters();
    const int frames = (pcm.size() - Window) / Hop + 1;
    out.reserve(frames);

    for (int f = 0; f < frames; ++f) {
        QVector<float> row;
        row.reserve(Bands);
        for (double hz : centers)
            row.append(float(std::log(goertzel(pcm.constData() + f * Hop, Window, hz) + 1e-9)));
        out.append(row);
    }

    // Cepstral mean normalisation: removes the microphone's colouring, which
    // differs between enrolment and daily use.
    for (int b = 0; b < Bands; ++b) {
        double mean = 0.0;
        for (const auto &row : out)
            mean += row[b];
        mean /= out.size();
        for (auto &row : out)
            row[b] = float(row[b] - mean);
    }
    return out;
}

double WakeModel::distance(const QVector<QVector<float>> &a, const QVector<QVector<float>> &b)
{
    if (a.isEmpty() || b.isEmpty())
        return 1e9;

    const int n = a.size();
    const int m = b.size();
    QVector<double> prev(m + 1, 1e9);
    QVector<double> cur(m + 1, 1e9);
    prev[0] = 0.0;

    for (int i = 1; i <= n; ++i) {
        cur[0] = 1e9;
        for (int j = 1; j <= m; ++j) {
            double cost = 0.0;
            for (int k = 0; k < Bands; ++k)
                cost += std::abs(double(a[i - 1][k]) - double(b[j - 1][k]));
            cost /= Bands;
            cur[j] = cost + std::min({prev[j], cur[j - 1], prev[j - 1]});
        }
        std::swap(prev, cur);
    }
    // Normalised by the path length so a longer recording is not punished.
    return prev[m] / (n + m);
}

double WakeModel::score(const QVector<qint16> &pcm) const
{
    const auto heard = features(pcm);
    if (heard.size() < MinFrames || m_templates.isEmpty())
        return 0.0;

    double best = 0.0;
    for (const auto &tpl : m_templates) {
        const double d = distance(heard, tpl);
        // Measured on the enrolment takes: same word ~0.0-0.75, ordinary
        // speech ~1.5. Tuned in phase 31, on real audio, not on a guess.
        const double s = std::exp(-0.8 * d);
        best = std::max(best, s);
    }
    return best;
}

bool WakeModel::enroll(const QVector<qint16> &pcm)
{
    const auto f = features(pcm);
    if (f.size() < MinFrames)
        return false;
    m_templates.append(f);
    return true;
}

double WakeModel::calibrate()
{
    if (m_templates.size() < 2)
        return m_threshold;

    double worst = 1.0;
    for (int i = 0; i < m_templates.size(); ++i)
        for (int j = i + 1; j < m_templates.size(); ++j)
            worst = std::min(worst, std::exp(-0.8 * distance(m_templates[i], m_templates[j])));

    // Sit below the weakest consistent take so a different word is refused,
    // but never below a gate that would ignore the owner's own voice.
    m_threshold = std::clamp(worst - 0.15, 0.35, 0.90);
    return m_threshold;
}

bool WakeModel::save(const QString &path, QString *error) const
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        if (error)
            *error = file.errorString();
        return false;
    }
    QTextStream out(&file);
    out << "newton-wake 1\n";
    out << "threshold " << m_threshold << '\n';
    out << "templates " << m_templates.size() << '\n';
    for (const auto &tpl : m_templates) {
        out << "frames " << tpl.size() << ' ' << Bands << '\n';
        for (const auto &row : tpl) {
            for (int i = 0; i < row.size(); ++i)
                out << (i ? " " : "") << row[i];
            out << '\n';
        }
    }
    file.close();
    return true;
}

WakeModel WakeModel::load(const QString &path, QString *error)
{
    WakeModel model;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        if (error)
            *error = file.errorString();
        return model;
    }
    QTextStream in(&file);
    QString magic;
    int version = 0;
    in >> magic >> version;
    if (magic != QStringLiteral("newton-wake") || version != 1) {
        if (error)
            *error = QStringLiteral("not a wake model");
        return model;
    }
    QString key;
    int stored = 0;
    in >> key >> model.m_threshold;
    in >> key >> stored;
    if (in.status() != QTextStream::Ok || stored < 0 || stored > 64) {
        if (error)
            *error = QStringLiteral("truncated wake model");
        model.m_templates.clear();
        return model;
    }
    for (int t = 0; t < stored; ++t) {
        int frames = 0;
        int bands = 0;
        in >> key >> frames >> bands;
        if (key != QStringLiteral("frames") || frames <= 0 || frames > 2000 || bands != Bands) {
            if (error)
                *error = QStringLiteral("bad template");
            model.m_templates.clear();
            return model;
        }
        QVector<QVector<float>> tpl;
        tpl.reserve(frames);
        for (int f = 0; f < frames; ++f) {
            QVector<float> row(Bands);
            for (int b = 0; b < Bands; ++b)
                in >> row[b];
            tpl.append(row);
        }
        model.m_templates.append(tpl);
    }
    return model;
}
