import { View, Text, FlatList, StyleSheet } from 'react-native';
import Card from '@/components/Card';

interface Item {
  id: string;
  title: string;
  description: string;
}

const ITEMS: Item[] = [
  { id: '1', title: 'First Item', description: 'This is the first item in the list' },
  { id: '2', title: 'Second Item', description: 'This is the second item in the list' },
  { id: '3', title: 'Third Item', description: 'This is the third item in the list' },
  { id: '4', title: 'Fourth Item', description: 'This is the fourth item in the list' },
  { id: '5', title: 'Fifth Item', description: 'This is the fifth item in the list' },
];

export default function ListScreen() {
  return (
    <View style={styles.container}>
      <Text style={styles.header}>Item List</Text>
      <FlatList
        data={ITEMS}
        keyExtractor={(item) => item.id}
        renderItem={({ item }) => (
          <Card
            title={item.title}
            description={item.description}
            onPress={() => console.log(`Pressed: ${item.title}`)}
          />
        )}
        contentContainerStyle={styles.list}
        showsVerticalScrollIndicator={false}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F5F5F5',
  },
  header: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#1A1A1A',
    padding: 20,
    paddingBottom: 12,
  },
  list: {
    paddingHorizontal: 20,
    paddingBottom: 20,
  },
});
