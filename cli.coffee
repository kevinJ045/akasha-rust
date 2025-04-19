using namespace std::ns ->
  define Main class
    @baseUrl = "https://akasha.cv/api"
    
    @apify: (name) ->
      @baseUrl + name
    
    @colors = 
      hydro: { r: 0, g: 144, b: 255 }
      pyro: { r: 255, g: 69, b: 0 }
      cryo: { r: 167, g: 223, b: 236 }
      electro: { r: 178, g: 132, b: 255 }
      anemo: { r: 148, g: 255, b: 198 }
      geo: { r: 255, g: 198, b: 93 }
      dendro: { r: 147, g: 215, b: 65 }
      default: { r: 255, g: 255, b: 255 }
    
    @colorize: (text, color) ->
      { r, g, b } = color or @colors.default
      "\x1b[38;2;#{r};#{g};#{b}m#{text}\x1b[0m"

    @formatNumber: (num) ->
      num.toFixed(2).replace(/\B(?=(\d{3})+(?!\d))/g, ",")

    @displayCharacterInfo: (char) ->
      print "\n" + "=".repeat(50)
      
      # Character Basic Info
      elementColor = @colors[char.characterMetadata.element.toLowerCase()] or @colors.default
      print @colorize(char.name, elementColor) + " (C#{char.constellation})"
      print "Level #{char.propMap.level.val}/#{char.propMap.ascension.val * 10}"
      
      # Talents
      print "\nTalents:"
      print "  Normal Attack: #{char.talentsLevelMap.normalAttacks.level}"
      print "  Elemental Skill: #{char.talentsLevelMap.elementalSkill.level}"
      print "  Elemental Burst: #{char.talentsLevelMap.elementalBurst.level}"
      
      # Stats
      print "\nStats:"
      print "  HP: #{@formatNumber(char.stats.maxHp.value)}"
      print "  ATK: #{@formatNumber(char.stats.atk.value)}"
      print "  DEF: #{@formatNumber(char.stats.def.value)}"
      print "  Crit Rate: #{(char.stats.critRate.value * 100).toFixed(1)}%"
      print "  Crit DMG: #{(char.stats.critDamage.value * 100).toFixed(1)}%"
      print "  Energy Recharge: #{(char.stats.energyRecharge.value * 100).toFixed(1)}%"
      print "  Elemental Mastery: #{Math.round(char.stats.elementalMastery.value)}"
      
      # Element-specific damage bonus
      for element in ['physical', 'geo', 'cryo', 'pyro', 'anemo', 'hydro', 'dendro', 'electro']
        bonus = char.stats[element + 'DamageBonus'].value
        if bonus > 0
          print "  #{element.charAt(0).toUpperCase() + element.slice(1)} DMG Bonus: #{(bonus * 100).toFixed(1)}%"

      # Weapon Info
      weapon = char.weapon
      print "\nWeapon: #{@colorize(weapon.name, { r: 255, g: 215, b: 0 })} R#{weapon.weaponInfo.refinementLevel.value + 1}"
      print "  Level #{weapon.weaponInfo.level}/#{weapon.weaponInfo.promoteLevel * 10}"

      # Artifacts
      print "\nArtifact Sets:"
      for setName, details of char.artifactSets
        print "  #{setName} (#{details.count}pc)"
      
      print "\nArtifact Main Stats:"
      mainStats = char.artifactObjects
      for piece, stats of mainStats
        pieceName = piece.replace('EQUIP_', '')
        print "  #{pieceName}: #{stats.mainStatKey}"

      # Build Quality
      print "\nBuild Quality:"
      print "  Crit Value: #{char.critValue.toFixed(2)}"

      # Calculations (if they exist from previous code)
      if char.calculations?.fit
        calc = char.calculations.fit
        print "\nBuild Analysis:"
        print "  #{calc.name}"
        print "  Details: #{calc.details}"
        print "  Result: #{@formatNumber(calc.result)}"
        if calc.ranking and calc.outOf
          percentage = ((calc.ranking / calc.outOf) * 100).toFixed(2)
          print "  Ranking: #{calc.ranking} out of #{calc.outOf} (Top #{percentage}%)"


    @getUserCalculations: (user_id) ->
      try
        response = wait curl @apify('/getCalculationsForUser/' + user_id), json: true
        return response.data
      catch e
        print "Error fetching user data: #{e.message}"
        return null

    @getUserBuilds: (user_id) ->
      try
        response = wait curl @apify('/builds/?sort=critValue&order=-1&size=20&page=1&filter=&uids=&p=&fromId=&li=&uid=' + user_id), json: true
        return response.data
      catch e
        print "Error fetching user data: #{e.message}"
        return null

    @main: (argv) ->
      uid = '772493838' || input 'Enter Genshin Impact UID: '
      
      print "Fetching data for UID #{uid}..."
      characters = @getUserBuilds(uid)
      calculations = @getUserCalculations(uid)
      
      if not characters or characters.length is 0
        print "No data found for this UID"
        return

      
      # Owner Info
      print "\nOwner Info:"
      print "  #{characters[0].owner.nickname} (AR#{Math.floor(characters[0].owner.adventureRank)})"
      print "  Server: #{characters[0].owner.region}"
        
      print "\nFound #{characters.length} characters"
      for char in characters
        @displayCharacterInfo({
          ...calculations.find((calculation) => calculation.characterId is char.characterId)
          ...char
        })
