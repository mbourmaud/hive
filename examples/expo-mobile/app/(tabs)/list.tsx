import { View, Text, FlatList, StyleSheet, TextInput, TouchableOpacity, Animated } from 'react-native';
import { useState, useMemo, useRef } from 'react';
import Card from '@/components/Card';

interface Item {
  id: string;
  title: string;
  description: string;
  category: 'work' | 'personal' | 'urgent';
}

const ITEMS: Item[] = [
  { id: '1', title: 'First Item', description: 'This is the first item in the list', category: 'work' },
  { id: '2', title: 'Second Item', description: 'This is the second item in the list', category: 'personal' },
  { id: '3', title: 'Third Item', description: 'This is the third item in the list', category: 'urgent' },
  { id: '4', title: 'Fourth Item', description: 'This is the fourth item in the list', category: 'work' },
  { id: '5', title: 'Fifth Item', description: 'This is the fifth item in the list', category: 'personal' },
];

type CategoryFilter = 'all' | 'work' | 'personal' | 'urgent';

export default function ListScreen() {
  const [searchQuery, setSearchQuery] = useState('');
  const [favorites, setFavorites] = useState<Set<string>>(new Set());
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>('all');

  const toggleFavorite = (id: string) => {
    setFavorites(prev => {
      const newFavorites = new Set(prev);
      if (newFavorites.has(id)) {
        newFavorites.delete(id);
      } else {
        newFavorites.add(id);
      }
      return newFavorites;
    });
  };

  // Filter items based on search query and category
  const filteredItems = useMemo(() => {
    let items = ITEMS;

    // Filter by category
    if (selectedCategory !== 'all') {
      items = items.filter(item => item.category === selectedCategory);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      items = items.filter(item =>
        item.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        item.description.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    return items;
  }, [searchQuery, selectedCategory]);

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'work': return '#007AFF';
      case 'personal': return '#34C759';
      case 'urgent': return '#FF3B30';
      default: return '#666';
    }
  };

  const getCategoryEmoji = (category: string) => {
    switch (category) {
      case 'work': return '💼';
      case 'personal': return '👤';
      case 'urgent': return '🔥';
      default: return '📋';
    }
  };

  return (
    <View style={styles.container}>
      <View style={styles.headerContainer}>
        <Text style={styles.header}>Item List</Text>
        <View style={styles.favoriteBadge}>
          <Text style={styles.favoriteBadgeText}>❤️ {favorites.size}</Text>
        </View>
      </View>

      {/* Search Bar */}
      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder="🔍 Search items..."
          value={searchQuery}
          onChangeText={setSearchQuery}
          placeholderTextColor="#999"
          autoCapitalize="none"
          autoCorrect={false}
        />
      </View>

      {/* Category Filters */}
      <View style={styles.categoryContainer}>
        {(['all', 'work', 'personal', 'urgent'] as CategoryFilter[]).map(category => (
          <TouchableOpacity
            key={category}
            style={[
              styles.categoryButton,
              selectedCategory === category && styles.categoryButtonActive,
              selectedCategory === category && { backgroundColor: getCategoryColor(category) }
            ]}
            onPress={() => setSelectedCategory(category)}
          >
            <Text style={[
              styles.categoryButtonText,
              selectedCategory === category && styles.categoryButtonTextActive
            ]}>
              {category === 'all' ? '📋 All' : `${getCategoryEmoji(category)} ${category.charAt(0).toUpperCase() + category.slice(1)}`}
            </Text>
          </TouchableOpacity>
        ))}
      </View>

      {/* Results count */}
      {searchQuery.trim() !== '' && (
        <Text style={styles.resultsText}>
          {filteredItems.length} {filteredItems.length === 1 ? 'result' : 'results'}
        </Text>
      )}

      <FlatList
        data={filteredItems}
        keyExtractor={(item) => item.id}
        renderItem={({ item }) => (
          <View style={styles.itemContainer}>
            <View style={styles.itemContent}>
              <View style={[styles.categoryTag, { backgroundColor: getCategoryColor(item.category) + '20' }]}>
                <Text style={[styles.categoryTagText, { color: getCategoryColor(item.category) }]}>
                  {getCategoryEmoji(item.category)}
                </Text>
              </View>
              <View style={styles.cardWrapper}>
                <Card
                  title={item.title}
                  description={item.description}
                  onPress={() => console.log(`Pressed: ${item.title}`)}
                />
              </View>
              <TouchableOpacity
                style={styles.favoriteButton}
                onPress={() => toggleFavorite(item.id)}
              >
                <Text style={styles.favoriteIcon}>
                  {favorites.has(item.id) ? '❤️' : '🤍'}
                </Text>
              </TouchableOpacity>
            </View>
          </View>
        )}
        contentContainerStyle={styles.list}
        showsVerticalScrollIndicator={false}
        ListEmptyComponent={
          <View style={styles.emptyContainer}>
            <Text style={styles.emptyText}>No items found</Text>
            <Text style={styles.emptySubtext}>Try a different search term or category</Text>
          </View>
        }
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F5F5F5',
  },
  headerContainer: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingBottom: 12,
  },
  header: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#1A1A1A',
  },
  favoriteBadge: {
    backgroundColor: '#FF3B30',
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 20,
  },
  favoriteBadgeText: {
    color: '#FFFFFF',
    fontSize: 14,
    fontWeight: '600',
  },
  searchContainer: {
    paddingHorizontal: 20,
    paddingBottom: 12,
  },
  searchInput: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 14,
    fontSize: 16,
    color: '#1A1A1A',
    borderWidth: 1,
    borderColor: '#E5E5E5',
  },
  categoryContainer: {
    flexDirection: 'row',
    paddingHorizontal: 20,
    paddingBottom: 12,
    gap: 8,
  },
  categoryButton: {
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: '#FFFFFF',
    borderWidth: 1,
    borderColor: '#E5E5E5',
  },
  categoryButtonActive: {
    borderColor: 'transparent',
  },
  categoryButtonText: {
    fontSize: 13,
    color: '#666666',
    fontWeight: '500',
  },
  categoryButtonTextActive: {
    color: '#FFFFFF',
    fontWeight: '600',
  },
  resultsText: {
    fontSize: 14,
    color: '#666666',
    paddingHorizontal: 20,
    paddingBottom: 8,
  },
  list: {
    paddingHorizontal: 20,
    paddingBottom: 20,
  },
  itemContainer: {
    marginBottom: 12,
  },
  itemContent: {
    position: 'relative',
  },
  categoryTag: {
    position: 'absolute',
    top: 8,
    left: 8,
    zIndex: 10,
    width: 28,
    height: 28,
    borderRadius: 14,
    alignItems: 'center',
    justifyContent: 'center',
  },
  categoryTagText: {
    fontSize: 14,
  },
  cardWrapper: {
    flex: 1,
  },
  favoriteButton: {
    position: 'absolute',
    top: 8,
    right: 8,
    zIndex: 10,
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: '#FFFFFF',
    alignItems: 'center',
    justifyContent: 'center',
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  favoriteIcon: {
    fontSize: 18,
  },
  emptyContainer: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyText: {
    fontSize: 18,
    fontWeight: '600',
    color: '#666666',
    marginBottom: 8,
  },
  emptySubtext: {
    fontSize: 14,
    color: '#999999',
  },
});
