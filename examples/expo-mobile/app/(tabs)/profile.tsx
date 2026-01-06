import { View, Text, StyleSheet, ScrollView, TouchableOpacity } from 'react-native';
import { useState } from 'react';
import Button from '@/components/Button';

export default function ProfileScreen() {
  const [level] = useState(7);
  const [xp] = useState(385);
  const [xpToNextLevel] = useState(500);

  const achievements = [
    { id: 1, icon: '🏆', name: 'First Win', unlocked: true },
    { id: 2, icon: '🔥', name: '7 Day Streak', unlocked: true },
    { id: 3, icon: '⭐', name: 'Super Star', unlocked: true },
    { id: 4, icon: '💎', name: 'Premium', unlocked: false },
    { id: 5, icon: '🎯', name: '100 Tasks', unlocked: false },
    { id: 6, icon: '👑', name: 'Master', unlocked: false },
  ];

  const xpPercentage = (xp / xpToNextLevel) * 100;

  return (
    <ScrollView style={styles.container}>
      {/* Profile Header */}
      <View style={styles.headerSection}>
        <View style={styles.avatarContainer}>
          <View style={styles.avatar}>
            <Text style={styles.avatarText}>JD</Text>
          </View>
          <View style={styles.levelBadge}>
            <Text style={styles.levelText}>Lvl {level}</Text>
          </View>
          <View style={styles.onlineIndicator} />
        </View>

        <Text style={styles.name}>John Doe</Text>
        <Text style={styles.email}>john.doe@example.com</Text>
        <View style={styles.memberBadge}>
          <Text style={styles.memberBadgeText}>✨ Premium Member</Text>
        </View>
      </View>

      {/* XP Progress */}
      <View style={styles.xpCard}>
        <View style={styles.xpHeader}>
          <Text style={styles.xpTitle}>Experience Points</Text>
          <Text style={styles.xpText}>{xp} / {xpToNextLevel} XP</Text>
        </View>
        <View style={styles.xpBarContainer}>
          <View style={[styles.xpBar, { width: `${xpPercentage}%` }]} />
        </View>
        <Text style={styles.xpSubtext}>
          {xpToNextLevel - xp} XP to level {level + 1}
        </Text>
      </View>

      {/* Stats Grid */}
      <View style={styles.statsContainer}>
        <View style={styles.stat}>
          <Text style={styles.statValue}>42</Text>
          <Text style={styles.statLabel}>Tasks</Text>
        </View>
        <View style={styles.stat}>
          <Text style={styles.statValue}>128</Text>
          <Text style={styles.statLabel}>Points</Text>
        </View>
        <View style={styles.stat}>
          <Text style={styles.statValue}>15</Text>
          <Text style={styles.statLabel}>Days</Text>
        </View>
        <View style={styles.stat}>
          <Text style={styles.statValue}>8</Text>
          <Text style={styles.statLabel}>Streak</Text>
        </View>
      </View>

      {/* Achievements */}
      <View style={styles.achievementsSection}>
        <Text style={styles.sectionTitle}>🏆 Achievements</Text>
        <View style={styles.achievementsGrid}>
          {achievements.map(achievement => (
            <View
              key={achievement.id}
              style={[
                styles.achievementCard,
                !achievement.unlocked && styles.achievementLocked
              ]}
            >
              <Text style={[
                styles.achievementIcon,
                !achievement.unlocked && styles.achievementIconLocked
              ]}>
                {achievement.icon}
              </Text>
              <Text style={[
                styles.achievementName,
                !achievement.unlocked && styles.achievementNameLocked
              ]}>
                {achievement.name}
              </Text>
              {achievement.unlocked && (
                <View style={styles.unlockedBadge}>
                  <Text style={styles.unlockedText}>✓</Text>
                </View>
              )}
            </View>
          ))}
        </View>
      </View>

      {/* Quick Stats */}
      <View style={styles.quickStatsSection}>
        <Text style={styles.sectionTitle}>📊 This Week</Text>
        <View style={styles.quickStatItem}>
          <Text style={styles.quickStatIcon}>✅</Text>
          <View style={styles.quickStatContent}>
            <Text style={styles.quickStatLabel}>Completed Tasks</Text>
            <Text style={styles.quickStatValue}>23 tasks</Text>
          </View>
        </View>
        <View style={styles.quickStatItem}>
          <Text style={styles.quickStatIcon}>⏱️</Text>
          <View style={styles.quickStatContent}>
            <Text style={styles.quickStatLabel}>Time Spent</Text>
            <Text style={styles.quickStatValue}>12.5 hours</Text>
          </View>
        </View>
        <View style={styles.quickStatItem}>
          <Text style={styles.quickStatIcon}>🎯</Text>
          <View style={styles.quickStatContent}>
            <Text style={styles.quickStatLabel}>Accuracy</Text>
            <Text style={styles.quickStatValue}>94%</Text>
          </View>
        </View>
      </View>

      {/* Actions */}
      <View style={styles.actions}>
        <Button
          title="Edit Profile"
          onPress={() => console.log('Edit Profile pressed')}
        />
        <Button
          title="Settings"
          onPress={() => console.log('Settings pressed')}
          variant="secondary"
        />
        <TouchableOpacity style={styles.logoutButton}>
          <Text style={styles.logoutText}>Sign Out</Text>
        </TouchableOpacity>
      </View>

      <View style={styles.bottomSpacer} />
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F5F5F5',
  },
  headerSection: {
    alignItems: 'center',
    paddingTop: 40,
    paddingBottom: 20,
  },
  avatarContainer: {
    position: 'relative',
    marginBottom: 16,
  },
  avatar: {
    width: 100,
    height: 100,
    borderRadius: 50,
    backgroundColor: '#007AFF',
    alignItems: 'center',
    justifyContent: 'center',
    borderWidth: 4,
    borderColor: '#FFFFFF',
  },
  avatarText: {
    fontSize: 36,
    fontWeight: 'bold',
    color: '#FFFFFF',
  },
  levelBadge: {
    position: 'absolute',
    bottom: 0,
    right: 0,
    backgroundColor: '#FFD700',
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 12,
    borderWidth: 2,
    borderColor: '#FFFFFF',
  },
  levelText: {
    fontSize: 12,
    fontWeight: 'bold',
    color: '#FFFFFF',
  },
  onlineIndicator: {
    position: 'absolute',
    top: 2,
    right: 2,
    width: 16,
    height: 16,
    borderRadius: 8,
    backgroundColor: '#34C759',
    borderWidth: 2,
    borderColor: '#FFFFFF',
  },
  name: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#1A1A1A',
    marginBottom: 4,
  },
  email: {
    fontSize: 16,
    color: '#666666',
    marginBottom: 12,
  },
  memberBadge: {
    backgroundColor: '#FFD700',
    paddingHorizontal: 16,
    paddingVertical: 6,
    borderRadius: 20,
  },
  memberBadgeText: {
    fontSize: 14,
    fontWeight: '600',
    color: '#FFFFFF',
  },
  xpCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 16,
    padding: 20,
    marginHorizontal: 20,
    marginBottom: 20,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  xpHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  xpTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#1A1A1A',
  },
  xpText: {
    fontSize: 14,
    fontWeight: '600',
    color: '#007AFF',
  },
  xpBarContainer: {
    height: 8,
    backgroundColor: '#E5E5E5',
    borderRadius: 4,
    marginBottom: 8,
    overflow: 'hidden',
  },
  xpBar: {
    height: '100%',
    backgroundColor: '#FFD700',
    borderRadius: 4,
  },
  xpSubtext: {
    fontSize: 12,
    color: '#666666',
  },
  statsContainer: {
    flexDirection: 'row',
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 20,
    marginHorizontal: 20,
    marginBottom: 20,
  },
  stat: {
    flex: 1,
    alignItems: 'center',
  },
  statValue: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#007AFF',
  },
  statLabel: {
    fontSize: 14,
    color: '#666666',
    marginTop: 4,
  },
  achievementsSection: {
    paddingHorizontal: 20,
    marginBottom: 20,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#1A1A1A',
    marginBottom: 16,
  },
  achievementsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  achievementCard: {
    width: '30%',
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 12,
    alignItems: 'center',
    position: 'relative',
    borderWidth: 2,
    borderColor: '#FFD700',
  },
  achievementLocked: {
    backgroundColor: '#F5F5F5',
    borderColor: '#E5E5E5',
  },
  achievementIcon: {
    fontSize: 32,
    marginBottom: 8,
  },
  achievementIconLocked: {
    opacity: 0.3,
  },
  achievementName: {
    fontSize: 11,
    fontWeight: '500',
    color: '#1A1A1A',
    textAlign: 'center',
  },
  achievementNameLocked: {
    color: '#999999',
  },
  unlockedBadge: {
    position: 'absolute',
    top: -6,
    right: -6,
    width: 20,
    height: 20,
    borderRadius: 10,
    backgroundColor: '#34C759',
    alignItems: 'center',
    justifyContent: 'center',
  },
  unlockedText: {
    fontSize: 12,
    color: '#FFFFFF',
    fontWeight: 'bold',
  },
  quickStatsSection: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 20,
    marginHorizontal: 20,
    marginBottom: 20,
  },
  quickStatItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#F5F5F5',
  },
  quickStatIcon: {
    fontSize: 28,
    marginRight: 16,
  },
  quickStatContent: {
    flex: 1,
  },
  quickStatLabel: {
    fontSize: 14,
    color: '#666666',
    marginBottom: 4,
  },
  quickStatValue: {
    fontSize: 16,
    fontWeight: '600',
    color: '#1A1A1A',
  },
  actions: {
    paddingHorizontal: 20,
    gap: 12,
  },
  logoutButton: {
    backgroundColor: '#FF3B30',
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
  },
  logoutText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#FFFFFF',
  },
  bottomSpacer: {
    height: 40,
  },
});
