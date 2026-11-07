#pragma once

#include <QString>
#include <QVector>

// The wake word, learned from the owner's voice instead of downloaded.
//
// There is no transcription here and no speech model: the utterance is cut into
// short spectra (log energies at 20 mel-spaced bands) and compared with the
// templates recorded at enrolment by dynamic time warping. That is enough to
// tell "Newton" from the rest of the room, and it runs in a few milliseconds.
class WakeModel
{
public:
    // Below this, an utterance is treated as noise rather than the wake word.
    static constexpr double DefaultThreshold = 0.50;

    bool isReady() const { return !m_templates.isEmpty(); }
    int templates() const { return m_templates.size(); }
    double threshold() const { return m_threshold; }
    void setThreshold(double threshold) { m_threshold = threshold; }

    // 0..1. Compares the utterance with every enrolled template, keeps the best.
    double score(const QVector<qint16> &pcm) const;

    // Adds one recording of the wake word. Utterances too short to be a word
    // are refused so a mis-tap cannot poison the model.
    bool enroll(const QVector<qint16> &pcm);

    // Sets the threshold from how closely the enrolled takes agree with each
    // other: a steady voice gets a strict gate, a variable one a looser gate.
    // Returns the threshold it chose.
    double calibrate();

    bool save(const QString &path, QString *error = nullptr) const;
    static WakeModel load(const QString &path, QString *error = nullptr);

    // Exposed for the test: the feature matrix of one utterance.
    static QVector<QVector<float>> features(const QVector<qint16> &pcm);
    static double distance(const QVector<QVector<float>> &a, const QVector<QVector<float>> &b);

private:
    QVector<QVector<QVector<float>>> m_templates;
    double m_threshold = DefaultThreshold;
};
