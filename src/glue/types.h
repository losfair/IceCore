#ifndef _ICE_GLUE_TYPES_H_
#define _ICE_GLUE_TYPES_H_

#include <unordered_map>

template<class K, class V> class Map : public std::unordered_map<K, V> {};

#endif
